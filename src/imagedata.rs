use fitsio::images::{ImageDescription, ImageType};
use fitsio::FitsFile;
use image::{ColorType, DynamicImage, ImageBuffer};
use log::warn;
use serde::{Deserialize, Serialize};
use serialimagedata::{ImageMetaData, SerialImageData, SerialImagePixel, SerialImageStorageTypes};
use std::fmt::Display;
use std::fs::remove_file;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// image crate re-exports.

#[derive(Clone)]
/// Image data structure
///
/// This structure contains the image data and the metadata associated with it.
/// This structure implements the [`std::fmt::Display`] and [`std::clone::Clone`] traits.
pub struct ImageData {
    img: DynamicImage,
    meta: ImageMetaData,
}

impl Display for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.meta)?;
        write!(f, "Size: {} x {}", self.img.width(), self.img.height())
    }
}

impl ImageData {
    /// Create a new image data structure from a `DynamicImage` and `ImageMetaData`.
    pub fn new(img: DynamicImage, meta: ImageMetaData) -> Self {
        Self { img, meta }
    }

    /// Add an extended attribute to the image metadata using `vec::push()`.
    ///
    /// # Panics
    /// If the new capacity exceeds `isize::MAX` bytes.
    pub fn add_extended_attrib(&mut self, key: &str, val: &str) {
        self.meta.add_extended_attrib(key, val);
    }

    /// Get the image metadata.
    pub fn get_metadata(&self) -> &ImageMetaData {
        &self.meta
    }

    /// Set the image metadata.
    pub fn set_metadata(&mut self, meta: ImageMetaData) {
        self.meta = meta;
    }

    /// Get the internal `image::DynamicImage` object from the image data structure.
    pub fn get_image(&self) -> &DynamicImage {
        &self.img
    }

    /// Get a mutable reference to the internal `image::DynamicImage` object from the image data structure.
    pub fn get_image_mut(&mut self) -> &mut DynamicImage {
        &mut self.img
    }

    /// Get the [`SerialImagePixel`] this [`ImageData`] structure can convert into.
    pub fn get_serial_pixel(self) -> Result<SerialImagePixel, &'static str> {
        self.img.color().try_into()
    }

    /// Find the optimum exposure time and binning to reach a target pixel value.
    ///
    /// # Arguments
    ///  * `percentile_pix` - The percentile of the pixel values to use as the target pixel value, in percentage.
    ///  * `pixel_tgt` - The target pixel value, in fraction.
    ///  * `pixel_uncertainty` - The uncertainty of the target pixel value, in fraction.
    ///  * `min_allowed_exp` - The minimum allowed exposure time.
    ///  * `max_allowed_exp` - The maximum allowed exposure time.
    ///  * `max_allowed_bin` - The maximum allowed binning.
    ///  * `pixel_exclusion` - The number of pixels to exclude from the top of the image.
    ///
    /// # Errors
    ///  - Errors are returned as strings.
    pub fn find_optimum_exposure(
        &self,
        percentile_pix: f32,
        pixel_tgt: f32,
        pixel_uncertainty: f32,
        min_allowed_exp: Duration,
        max_allowed_exp: Duration,
        max_allowed_bin: u16,
        pixel_exclusion: u32,
    ) -> Result<(Duration, u16), String> {
        let exposure = self.meta.exposure;

        let mut target_exposure;

        let mut change_bin = true;

        if pixel_tgt < 1.6e-5f32 || pixel_tgt > 1f32 {
            return Err("Target pixel value must be between 1.6e-5 and 1".to_string());
        }

        if pixel_uncertainty < 1.6e-5f32 || pixel_uncertainty > 1f32 {
            return Err("Pixel uncertainty must be between 1.6e-5 and 1".to_string());
        }

        if percentile_pix < 0f32 || percentile_pix > 100f32 {
            return Err("Percentile must be between 0 and 100".to_string());
        }

        if min_allowed_exp >= max_allowed_exp {
            return Err(
                "Minimum allowed exposure must be less than maximum allowed exposure".to_string(),
            );
        }

        let max_allowed_bin = if max_allowed_bin < 2 {
            1
        } else {
            max_allowed_bin
        };

        let pixel_tgt = pixel_tgt * 65535f32;
        let pixel_uncertainty = pixel_uncertainty * 65535f32;

        if self.meta.bin_x != self.meta.bin_y || max_allowed_bin < 2 {
            change_bin = false;
        }
        let mut bin = self.meta.bin_x as u16;
        let mut img = self.img.clone().into_luma16();
        img.sort();
        let mut coord: usize;
        if percentile_pix > 99.9 {
            coord = img.len() - 1 as usize;
        } else {
            coord = (percentile_pix * (img.len() - 1) as f32 * 0.01).floor() as usize;
        }
        if coord < pixel_exclusion as usize {
            coord = img.len() - 1 - pixel_exclusion as usize;
        }
        let imgvec = img.to_vec();
        let val = imgvec.get(coord);
        let val = match val {
            Some(v) => *v as f64,
            None => {
                warn!("Could not get pixel value at {} percentile", percentile_pix);
                1e-5 as f64
            }
        };

        if (pixel_tgt as f64 - val).abs() < pixel_uncertainty as f64 {
            return Ok((exposure, bin));
        }

        let val = {
            if val <= 1e-5 {
                1e-5
            } else {
                val
            }
        };

        target_exposure = Duration::from_secs_f64(
            (pixel_tgt as f64 * exposure.as_micros() as f64 * 1e-6 / val as f64).abs(),
        );

        if change_bin {
            let mut tgt_exp = target_exposure;
            let mut bin_ = bin;
            if tgt_exp < max_allowed_exp {
                while tgt_exp < max_allowed_exp && bin_ > 2 {
                    bin_ /= 2;
                    tgt_exp *= 4;
                }
            } else {
                while tgt_exp > max_allowed_exp && bin_ * 2 <= max_allowed_bin {
                    bin_ *= 2;
                    tgt_exp /= 4;
                }
            }
            target_exposure = tgt_exp;
            bin = bin_;
        }

        if target_exposure > max_allowed_exp {
            target_exposure = max_allowed_exp;
        }

        if target_exposure < min_allowed_exp {
            target_exposure = min_allowed_exp;
        }

        if bin < 1 {
            bin = 1;
        }
        if bin > max_allowed_bin {
            bin = max_allowed_bin;
        }

        Ok((target_exposure, bin))
    }

    /// Save the image data to a FITS file.
    ///
    /// # Arguments
    ///  * `dir_prefix` - The directory where the file will be saved.
    ///  * `file_prefix` - The prefix of the file name. The file name will be of the form `{file_prefix}_timestamp.fits`.
    ///  * `progname` - The name of the program that generated the image.
    ///  * `compress` - Whether to compress the FITS file.
    ///  * `overwrite` - Whether to overwrite the file if it already exists.
    ///
    /// # Errors
    ///  * `fitsio::errors::Error::Message` with the error description.
    pub fn save_fits(
        &self,
        dir_prefix: &Path,
        file_prefix: &str,
        progname: &str,
        compress: bool,
        overwrite: bool,
    ) -> Result<(), fitsio::errors::Error> {
        if !dir_prefix.exists() {
            return Err(fitsio::errors::Error::Message(format!(
                "Directory {} does not exist",
                dir_prefix.to_string_lossy()
            )));
        }

        let timestamp;
        if let Ok(val) = self.meta.timestamp.duration_since(UNIX_EPOCH) {
            timestamp = val.as_millis()
        } else {
            return Err(fitsio::errors::Error::Message(format!(
                "Could not convert timestamp {:?} to milliseconds",
                self.meta.timestamp
            )));
        };

        let file_prefix = if file_prefix.trim().is_empty() {
            if self.meta.camera_name.is_empty() {
                "image"
            } else {
                self.meta.camera_name.as_str()
            }
        } else {
            file_prefix
        };

        let fpath = dir_prefix.join(Path::new(&format!(
            "{}_{}.fits",
            file_prefix, timestamp as u64
        )));

        if fpath.exists() {
            warn!("File {} already exists", fpath.to_string_lossy());
            if !overwrite {
                return Err(fitsio::errors::Error::Message(format!(
                    "File {:?} already exists",
                    fpath
                )));
            } else {
                warn!("Overwriting file {:?}", fpath);
                let res = remove_file(fpath.clone());
                if let Err(msg) = res {
                    return Err(fitsio::errors::Error::Message(format!(
                        "Could not remove file {:?}: {:?}",
                        fpath, msg
                    )));
                }
            }
        }

        let imgtype = self.img.color();
        let width = self.img.width();
        let height = self.img.height();
        let imgsize = [height as usize, width as usize];
        let data_type: ImageType;
        let primary_name: &str;

        match imgtype {
            ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8 => {
                data_type = ImageType::UnsignedByte;
            }
            ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16 => {
                data_type = ImageType::UnsignedShort;
            }
            ColorType::Rgb32F | ColorType::Rgba32F => {
                data_type = ImageType::Float;
            }
            _ => {
                return Err(fitsio::errors::Error::Message(format!(
                    "Unsupported image type {:?}",
                    imgtype
                )));
            }
        };

        match imgtype {
            ColorType::L8 | ColorType::L16 => {
                primary_name = "IMAGE";
            }
            ColorType::La8 | ColorType::La16 => {
                primary_name = "LUMA";
            }
            ColorType::Rgb8
            | ColorType::Rgb16
            | ColorType::Rgb32F
            | ColorType::Rgba8
            | ColorType::Rgba16
            | ColorType::Rgba32F => {
                primary_name = "RED";
            }
            _ => {
                return Err(fitsio::errors::Error::Message(format!(
                    "Unsupported image type {:?}",
                    imgtype
                )));
            }
        }

        let img_desc = ImageDescription {
            data_type,
            dimensions: &imgsize,
        };
        let path = Path::new(dir_prefix).join(Path::new(&format!(
            "{}_{}.fits{}",
            file_prefix,
            timestamp as u64,
            if compress { "[compress]" } else { "" }
        )));
        let mut fptr = FitsFile::create(path).open()?;

        let hdu = fptr.create_image(primary_name, &img_desc)?;
        match imgtype {
            ColorType::L8 => {
                hdu.write_image(&mut fptr, self.img.to_luma8().as_raw())?;
                hdu.write_key(&mut fptr, "CHANNELS", 1)?;
            }
            ColorType::L16 => {
                hdu.write_image(&mut fptr, self.img.to_luma16().as_raw())?;
                hdu.write_key(&mut fptr, "CHANNELS", 1)?;
            }
            ColorType::La8 => {
                self.write_la8(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::La16 => {
                self.write_la16(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgb8 => {
                self.write_rgb8(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgb16 => {
                self.write_rgb16(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgb32F => {
                self.write_rgb32(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgba8 => {
                self.write_rgba8(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgba16 => {
                self.write_rgba16(&hdu, &mut fptr, &img_desc)?;
            }
            ColorType::Rgba32F => {
                self.write_rgba32(&hdu, &mut fptr, &img_desc)?;
            }
            _ => {
                return Err(fitsio::errors::Error::Message(format!(
                    "Unsupported image type {:?}",
                    imgtype
                )));
            }
        }
        hdu.write_key(&mut fptr, "PROGRAM", progname)?;
        hdu.write_key(&mut fptr, "CAMERA", self.meta.camera_name.as_str())?;
        hdu.write_key(&mut fptr, "TIMESTAMP", timestamp as u64)?;
        hdu.write_key(&mut fptr, "CCDTEMP", self.meta.temperature)?;
        hdu.write_key(
            &mut fptr,
            "EXPOSURE_US",
            self.meta.exposure.as_micros() as u64,
        )?;
        hdu.write_key(&mut fptr, "ORIGIN_X", self.meta.img_left)?;
        hdu.write_key(&mut fptr, "ORIGIN_Y", self.meta.img_top)?;
        hdu.write_key(&mut fptr, "BINX", self.meta.bin_x)?;
        hdu.write_key(&mut fptr, "BINY", self.meta.bin_y)?;
        hdu.write_key(&mut fptr, "GAIN", self.meta.gain)?;
        hdu.write_key(&mut fptr, "OFFSET", self.meta.offset)?;
        hdu.write_key(&mut fptr, "GAIN_MIN", self.meta.min_gain)?;
        hdu.write_key(&mut fptr, "GAIN_MAX", self.meta.max_gain)?;
        for obj in self.meta.get_extended_data().iter() {
            hdu.write_key(&mut fptr, &obj.0, obj.1.as_str())?;
        }

        Ok(())
    }

    fn write_la8(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_luma_alpha8();
        let pixels = dat.pixels();
        let luma = pixels.map(|p| p[0]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let alpha = pixels.map(|p| p[1]).collect::<Vec<u8>>();
        hdu.write_image(fptr, luma.as_ref())?;
        let ahdu = fptr.create_image("ALPHA".to_string(), &img_desc)?;
        ahdu.write_image(fptr, alpha.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 2)?;
        Ok(())
    }

    fn write_la16(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_luma_alpha16();
        let pixels = dat.pixels();
        let luma = pixels.map(|p| p[0]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let alpha = pixels.map(|p| p[1]).collect::<Vec<u16>>();
        hdu.write_image(fptr, luma.as_ref())?;
        let ahdu = fptr.create_image("ALPHA".to_string(), &img_desc)?;
        ahdu.write_image(fptr, alpha.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 2)?;
        Ok(())
    }

    fn write_rgb8(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgb8();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<u8>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 3)?;
        Ok(())
    }

    fn write_rgb16(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgb16();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<u16>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 3)?;
        Ok(())
    }

    fn write_rgb32(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgb32f();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<f32>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<f32>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<f32>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 3)?;
        Ok(())
    }

    fn write_rgba8(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgba8();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<u8>>();
        let pixels = dat.pixels();
        let alpha = pixels.map(|p| p[3]).collect::<Vec<u8>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        let ahdu = fptr.create_image("ALPHA".to_string(), &img_desc)?;
        ahdu.write_image(fptr, alpha.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 4)?;
        Ok(())
    }

    fn write_rgba16(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgba16();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<u16>>();
        let pixels = dat.pixels();
        let alpha = pixels.map(|p| p[3]).collect::<Vec<u16>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        let ahdu = fptr.create_image("ALPHA".to_string(), &img_desc)?;
        ahdu.write_image(fptr, alpha.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 4)?;
        Ok(())
    }

    fn write_rgba32(
        &self,
        hdu: &fitsio::hdu::FitsHdu,
        fptr: &mut FitsFile,
        img_desc: &ImageDescription,
    ) -> Result<(), fitsio::errors::Error> {
        let dat = self.img.to_rgb32f();
        let pixels = dat.pixels();
        let red = pixels.map(|p| p[0]).collect::<Vec<f32>>();
        let pixels = dat.pixels();
        let green = pixels.map(|p| p[1]).collect::<Vec<f32>>();
        let pixels = dat.pixels();
        let blue = pixels.map(|p| p[2]).collect::<Vec<f32>>();
        let pixels = dat.pixels();
        let alpha = pixels.map(|p| p[3]).collect::<Vec<f32>>();
        hdu.write_image(fptr, red.as_ref())?;
        let ghdu = fptr.create_image("GREEN".to_string(), &img_desc)?;
        ghdu.write_image(fptr, green.as_ref())?;
        let bhdu = fptr.create_image("BLUE".to_string(), &img_desc)?;
        bhdu.write_image(fptr, blue.as_ref())?;
        let ahdu = fptr.create_image("ALPHA".to_string(), &img_desc)?;
        ahdu.write_image(fptr, alpha.as_ref())?;
        hdu.write_key(fptr, "CHANNELS", 4)?;
        Ok(())
    }
}

impl Serialize for ImageData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let color = self.img.color();
        let out_color: SerialImagePixel = color.try_into().map_err(serde::ser::Error::custom)?;
        match out_color {
            SerialImagePixel::U8(_) => {
                let res: Result<SerialImageData<u8>, &'static str> = self.try_into();
                res.serialize(serializer)
            }
            SerialImagePixel::U16(_) => {
                let res: Result<SerialImageData<u16>, &'static str> = self.try_into();
                res.serialize(serializer)
            }
            SerialImagePixel::F32(_) => {
                let res: Result<SerialImageData<f32>, &'static str> = self.try_into();
                res.serialize(serializer)
            }
        }
    }
}

impl TryFrom<ImageData> for SerialImageData<u8> {
    type Error = &'static str;
    fn try_from(value: ImageData) -> Result<SerialImageData<u8>, &'static str> {
        let img = value.img;
        let meta = value.meta;
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel: Result<SerialImagePixel, &'static str> = color.try_into();
        let pixel = match pixel {
            Ok(p) => p,
            Err(msg) => {
                return Err(msg);
            }
        };
        let imgdata = match color {
            ColorType::L8 => {
                let img = img.into_luma8();
                img.into_raw()
            }
            ColorType::Rgb8 => {
                let img = img.into_rgb8();
                img.into_raw()
            }
            ColorType::Rgba8 => {
                let img = img.into_rgba8();
                img.into_raw()
            }
            ColorType::La8 => {
                let img = img.into_luma_alpha8();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<&ImageData> for SerialImageData<u8> {
    type Error = &'static str;
    fn try_from(value: &ImageData) -> Result<SerialImageData<u8>, &'static str> {
        let img = value.img.clone();
        let meta = value.meta.clone();
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = color.try_into()?;
        let imgdata = match color {
            ColorType::L8 => {
                let img = img.into_luma8();
                img.into_raw()
            }
            ColorType::Rgb8 => {
                let img = img.into_rgb8();
                img.into_raw()
            }
            ColorType::Rgba8 => {
                let img = img.into_rgba8();
                img.into_raw()
            }
            ColorType::La8 => {
                let img = img.into_luma_alpha8();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<ImageData> for SerialImageData<u16> {
    type Error = &'static str;
    fn try_from(value: ImageData) -> Result<SerialImageData<u16>, &'static str> {
        let img = value.img.clone();
        let meta = value.meta.clone();
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = color.try_into()?;
        let imgdata = match color {
            ColorType::L16 => {
                let img = img.into_luma16();
                img.into_raw()
            }
            ColorType::Rgb16 => {
                let img = img.into_rgb16();
                img.into_raw()
            }
            ColorType::Rgba16 => {
                let img = img.into_rgba16();
                img.into_raw()
            }
            ColorType::La16 => {
                let img = img.into_luma_alpha16();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<&ImageData> for SerialImageData<u16> {
    type Error = &'static str;
    fn try_from(value: &ImageData) -> Result<SerialImageData<u16>, &'static str> {
        let img = value.img.clone();
        let meta = value.meta.clone();
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = color.try_into()?;
        let imgdata = match color {
            ColorType::L16 => {
                let img = img.into_luma16();
                img.into_raw()
            }
            ColorType::Rgb16 => {
                let img = img.into_rgb16();
                img.into_raw()
            }
            ColorType::Rgba16 => {
                let img = img.into_rgba16();
                img.into_raw()
            }
            ColorType::La16 => {
                let img = img.into_luma_alpha16();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<ImageData> for SerialImageData<f32> {
    type Error = &'static str;
    fn try_from(value: ImageData) -> Result<SerialImageData<f32>, &'static str> {
        let img = value.img;
        let meta = value.meta;
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = color.try_into()?;
        let imgdata = match color {
            ColorType::Rgb32F => {
                let img = img.into_rgb32f();
                img.into_raw()
            }
            ColorType::Rgba32F => {
                let img = img.into_rgba32f();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<&ImageData> for SerialImageData<f32> {
    type Error = &'static str;
    fn try_from(value: &ImageData) -> Result<SerialImageData<f32>, &'static str> {
        let img = value.img.clone();
        let meta = value.meta.clone();
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = color.try_into()?;
        let imgdata = match color {
            ColorType::Rgb32F => {
                let img = img.into_rgb32f();
                img.into_raw()
            }
            ColorType::Rgba32F => {
                let img = img.into_rgba32f();
                img.into_raw()
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(SerialImageData::new(
            meta,
            imgdata,
            width as usize,
            height as usize,
            pixel,
        ))
    }
}

impl TryFrom<SerialImageData<u8>> for ImageData {
    type Error = &'static str;
    fn try_from(value: SerialImageData<u8>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata();
        let imgdata = value.get_data().clone();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img = match color {
            ColorType::L8 => {
                let img = image::GrayImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image L8 image")?;
                DynamicImage::ImageLuma8(img)
            }
            ColorType::Rgb8 => {
                let img = image::RgbImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgb8 image")?;
                DynamicImage::ImageRgb8(img)
            }
            ColorType::Rgba8 => {
                let img = image::RgbaImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgba8 image")?;
                DynamicImage::ImageRgba8(img)
            }
            ColorType::La8 => {
                let img = image::GrayAlphaImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image La8 image")?;
                DynamicImage::ImageLumaA8(img)
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(ImageData::new(img, meta.clone()))
    }
}

impl TryFrom<&SerialImageData<u8>> for ImageData {
    type Error = &'static str;
    fn try_from(value: &SerialImageData<u8>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata().clone();
        let imgdata = value.get_data().clone();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img = match color {
            ColorType::L8 => {
                let img = image::GrayImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image L8 image")?;
                DynamicImage::ImageLuma8(img)
            }
            ColorType::Rgb8 => {
                let img = image::RgbImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgb8 image")?;
                DynamicImage::ImageRgb8(img)
            }
            ColorType::Rgba8 => {
                let img = image::RgbaImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgba8 image")?;
                DynamicImage::ImageRgba8(img)
            }
            ColorType::La8 => {
                let img = image::GrayAlphaImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image La8 image")?;
                DynamicImage::ImageLumaA8(img)
            }
            _ => {
                return Err("Unsupported image type");
            }
        };
        Ok(ImageData::new(img, meta))
    }
}

impl TryFrom<SerialImageData<u16>> for ImageData {
    type Error = &'static str;
    fn try_from(value: SerialImageData<u16>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata();
        let imgdata = value.get_data();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img =
            match color {
                ColorType::L16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_luma16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgb16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgba16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgba<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgba16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::La16 => {
                    let mut img =
                        DynamicImage::from(ImageBuffer::<image::LumaA<u16>, Vec<u16>>::new(
                            width as u32,
                            height as u32,
                        ));
                    let imgbuf = img
                        .as_mut_luma_alpha16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                _ => {
                    return Err("Unsupported image type");
                }
            };
        Ok(ImageData::new(img, meta.clone()))
    }
}

impl TryFrom<&SerialImageData<u16>> for ImageData {
    type Error = &'static str;
    fn try_from(value: &SerialImageData<u16>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata().clone();
        let imgdata = value.get_data().clone();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img =
            match color {
                ColorType::L16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_luma16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgb16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgba16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgba<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgba16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::La16 => {
                    let mut img =
                        DynamicImage::from(ImageBuffer::<image::LumaA<u16>, Vec<u16>>::new(
                            width as u32,
                            height as u32,
                        ));
                    let imgbuf = img
                        .as_mut_luma_alpha16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                _ => {
                    return Err("Unsupported image type");
                }
            };
        Ok(ImageData::new(img, meta))
    }
}

impl TryFrom<SerialImageData<f32>> for ImageData {
    type Error = &'static str;
    fn try_from(value: SerialImageData<f32>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata();
        let imgdata = value.get_data();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img =
            match color {
                ColorType::Rgb32F => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<f32>, Vec<f32>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb32f()
                        .ok_or("Could not create image Rgb32F image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgba32F => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgba<f32>, Vec<f32>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgba32f()
                        .ok_or("Could not create image Rgba32F image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                _ => {
                    return Err("Unsupported image type");
                }
            };
        Ok(ImageData::new(img, meta.clone()))
    }
}

impl TryFrom<&SerialImageData<f32>> for ImageData {
    type Error = &'static str;
    fn try_from(value: &SerialImageData<f32>) -> Result<ImageData, &'static str> {
        let meta = value.get_metadata().clone();
        let imgdata = value.get_data().clone();
        let width = value.width();
        let height = value.height();
        let color = value.pixel().try_into()?;

        let img =
            match color {
                ColorType::Rgb32F => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<f32>, Vec<f32>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb32f()
                        .ok_or("Could not create image Rgb32F image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                ColorType::Rgba32F => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgba<f32>, Vec<f32>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgba32f()
                        .ok_or("Could not create image Rgba32F image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                _ => {
                    return Err("Unsupported image type");
                }
            };
        Ok(ImageData::new(img, meta))
    }
}
