use fitsio::images::{ImageDescription, ImageType};
use fitsio::FitsFile;
use image::{DynamicImage, ImageBuffer};
use log::warn;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::remove_file;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
#[deny(missing_docs)]
/// Image metadata structure.
/// This structure implements the [`std::fmt::Display`] and [`std::clone::Clone`] traits.
pub struct ImageMetaData {
    /// Binning in X direction
    pub bin_x: u32,
    /// Binning in Y direction
    pub bin_y: u32,
    /// Top of image (pixels, binned coordinates)
    pub img_top: u32,
    /// Left of image (pixels, binned coordinates)
    pub img_left: u32,
    /// Camera temperature (C)
    pub temperature: f32,
    /// Exposure time
    pub exposure: Duration,
    /// Timestamp of the image
    pub timestamp: SystemTime,
    /// Name of the camera
    pub camera_name: String,
    /// Gain (raw)
    pub gain: i64,
    /// Offset (raw)
    pub offset: i64,
    /// Minimum gain (raw)
    pub min_gain: i32,
    /// Maximum gain (raw)
    pub max_gain: i32,
    extended_metadata: Vec<(String, String)>,
}

impl ImageMetaData {
    /// Create a new image metadata structure.
    pub fn new(
        timestamp: SystemTime,
        exposure: Duration,
        temperature: f32,
        bin_x: u32,
        bin_y: u32,
        camera_name: &str,
        gain: i64,
        offset: i64,
    ) -> Self {
        Self {
            bin_x,
            bin_y,
            img_top: 0,
            img_left: 0,
            temperature,
            exposure,
            timestamp,
            camera_name: camera_name.to_string(),
            gain,
            offset,
            ..Default::default()
        }
    }

    /// Create a new image metadata structure with full parameters.
    pub fn full_builder(
        bin_x: u32,
        bin_y: u32,
        img_top: u32,
        img_left: u32,
        temperature: f32,
        exposure: Duration,
        timestamp: SystemTime,
        camera_name: &str,
        gain: i64,
        offset: i64,
        min_gain: i32,
        max_gain: i32,
    ) -> Self {
        Self {
            bin_x,
            bin_y,
            img_top,
            img_left,
            temperature,
            exposure,
            timestamp,
            camera_name: camera_name.to_string(),
            gain,
            offset,
            min_gain,
            max_gain,
            ..Default::default()
        }
    }
}

impl Default for ImageMetaData {
    fn default() -> Self {
        Self {
            bin_x: 1,
            bin_y: 1,
            img_top: 0,
            img_left: 0,
            temperature: 0f32,
            exposure: Duration::from_secs(0),
            timestamp: UNIX_EPOCH,
            camera_name: String::new(),
            gain: 0,
            offset: 0,
            min_gain: 0,
            max_gain: 0,
            extended_metadata: Vec::new(),
        }
    }
}

impl Display for ImageMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImageMetaData [{:#?}]:\n
            \tCamera name: {}\n
            \tImage Bin: {} x {}\n
            \tImage Origin: {} x {}
            \tExposure: {} s\n
            \tGain: {}, Offset: {}\n
            \tTemperature: {} C\n",
            self.timestamp,
            self.camera_name,
            self.bin_x,
            self.bin_y,
            self.img_left,
            self.img_top,
            self.exposure.as_secs(),
            self.gain,
            self.offset,
            self.temperature
        )?;
        if self.extended_metadata.len() > 0 {
            write!(f, "\tExtended Metadata:\n")?;
            for obj in self.extended_metadata.iter() {
                write!(f, "\t\t{}: {}\n", obj.0, obj.1)?;
            }
        };
        Ok(())
    }
}

impl ImageMetaData {
    /// Add an extended attribute to the image metadata using [`std::alloc::vec::push()`].
    ///
    /// # Panics
    ///
    /// If the new capacity exceeds `isize::MAX` bytes.
    pub fn add_extended_attrib(&mut self, key: &str, val: &str) {
        self.extended_metadata
            .push((key.to_string(), val.to_string()));
    }
}

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

/// Valid types for the serial image data structure: [`u8`], [`u16`], [`f32`].
pub trait SerialImageValidTypes {}

impl SerialImageValidTypes for u8 {}
impl SerialImageValidTypes for u16 {}
impl SerialImageValidTypes for f32 {}

#[derive(Clone, Serialize, Deserialize)]
/// Serializable Image Data Structure.
///
/// This structure is derived from the [`ImageData`] structure and is used to serialize the image data.
/// This structure implements the [`std::clone::Clone`] trait, as well as the [`std::convert::TryFrom`] and [`std::convert::TryInto`] traits.
pub struct SerialImageData<T: SerialImageValidTypes> {
    meta: ImageMetaData,
    imgdata: Vec<T>,
    width: usize,
    height: usize,
    pixel: usize,
}

impl<T: SerialImageValidTypes> SerialImageData<T> {
    pub fn new(
        meta: ImageMetaData,
        imgdata: Vec<T>,
        width: usize,
        height: usize,
        pixel: usize,
    ) -> Self {
        Self {
            meta,
            imgdata,
            width,
            height,
            pixel,
        }
    }

    pub fn get_metadata(&self) -> &ImageMetaData {
        &self.meta
    }

    pub fn get_image_data(&self) -> &Vec<T> {
        &self.imgdata
    }

    pub fn get_image_data_mut(&mut self) -> &mut Vec<T> {
        &mut self.imgdata
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_pixel(&self) -> usize {
        self.pixel
    }
}

impl ImageData {
    /// Create a new image data structure from a `DynamicImage` and `ImageMetaData`.
    pub fn new(img: DynamicImage, meta: ImageMetaData) -> Self {
        Self { img, meta }
    }

    /// Add an extended attribute to the image metadata using [`std::alloc::vec::push()`].
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

    /// Find the optimum exposure time and binning to reach a target pixel value.
    ///
    /// # Arguments
    ///  * `percentile_pix` - The percentile of the pixel values to use as the target pixel value, in fraction.
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
    /// Note: Only 8 or 16 bit grayscale images are supported. That is, the underlying image data must have color types [`image::ColorType::L8`] or [`image::ColorType::L16`].
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

        match imgtype {
            image::ColorType::L8 => {
                data_type = ImageType::UnsignedByte;
            }
            image::ColorType::L16 => {
                data_type = ImageType::UnsignedShort;
            }
            _ => {
                return Err(fitsio::errors::Error::Message(format!(
                    "Unsupported image type {:?}",
                    imgtype
                )));
            }
        };

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

        let hdu = fptr.create_image("IMAGE".to_string(), &img_desc)?;
        match imgtype {
            image::ColorType::L8 => {
                hdu.write_image(&mut fptr, self.img.to_luma8().as_raw())?;
            }
            image::ColorType::L16 => {
                hdu.write_image(&mut fptr, self.img.to_luma16().as_raw())?;
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
        for obj in self.meta.extended_metadata.iter() {
            hdu.write_key(&mut fptr, &obj.0, obj.1.as_str())?;
        }

        Ok(())
    }
}

fn usize_from_color(value: image::ColorType) -> Option<usize> {
    match value {
        image::ColorType::L8 => Some(1),
        image::ColorType::L16 => Some(2),
        image::ColorType::Rgb8 => Some(3),
        image::ColorType::Rgba8 => Some(4),
        image::ColorType::Rgb16 => Some(5),
        image::ColorType::Rgba16 => Some(6),
        image::ColorType::La16 => Some(7),
        image::ColorType::La8 => Some(8),
        image::ColorType::Rgb32F => Some(9),
        image::ColorType::Rgba32F => Some(10),
        _ => None,
    }
}

fn color_from_usize(value: usize) -> Option<image::ColorType> {
    match value {
        1 => Some(image::ColorType::L8),
        2 => Some(image::ColorType::L16),
        3 => Some(image::ColorType::Rgb8),
        4 => Some(image::ColorType::Rgba8),
        5 => Some(image::ColorType::Rgb16),
        6 => Some(image::ColorType::Rgba16),
        7 => Some(image::ColorType::La16),
        8 => Some(image::ColorType::La8),
        9 => Some(image::ColorType::Rgb32F),
        10 => Some(image::ColorType::Rgba32F),
        _ => None,
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
        let pixel = usize_from_color(color).ok_or("Unsupported image type")?;
        let imgdata = match color {
            image::ColorType::L8 => {
                let img = img.into_luma8();
                img.into_raw()
            }
            image::ColorType::Rgb8 => {
                let img = img.into_rgb8();
                img.into_raw()
            }
            image::ColorType::Rgba8 => {
                let img = img.into_rgba8();
                img.into_raw()
            }
            image::ColorType::La8 => {
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
        let img = value.img;
        let meta = value.meta;
        let color = img.color();
        let width = img.width();
        let height = img.height();
        let pixel = usize_from_color(color).ok_or("Unsupported image type")?;
        let imgdata = match color {
            image::ColorType::L16 => {
                let img = img.into_luma16();
                img.into_raw()
            }
            image::ColorType::Rgb16 => {
                let img = img.into_rgb16();
                img.into_raw()
            }
            image::ColorType::Rgba16 => {
                let img = img.into_rgba16();
                img.into_raw()
            }
            image::ColorType::La16 => {
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
        let pixel = usize_from_color(color).ok_or("Unsupported image type")?;
        let imgdata = match color {
            image::ColorType::Rgb32F => {
                let img = img.into_rgb32f();
                img.into_raw()
            }
            image::ColorType::Rgba32F => {
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
        let meta = value.meta;
        let imgdata = value.imgdata;
        let width = value.width;
        let height = value.height;
        let pixel = value.pixel;
        let color = color_from_usize(pixel).ok_or("Unsupported image type")?;
        let img = match color {
            image::ColorType::L8 => {
                let img = image::GrayImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image L8 image")?;
                DynamicImage::ImageLuma8(img)
            }
            image::ColorType::Rgb8 => {
                let img = image::RgbImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgb8 image")?;
                DynamicImage::ImageRgb8(img)
            }
            image::ColorType::Rgba8 => {
                let img = image::RgbaImage::from_vec(width as u32, height as u32, imgdata)
                    .ok_or("Could not create image Rgba8 image")?;
                DynamicImage::ImageRgba8(img)
            }
            image::ColorType::La8 => {
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
        let meta = value.meta;
        let imgdata = value.imgdata;
        let width = value.width;
        let height = value.height;
        let pixel = value.pixel;
        let color = color_from_usize(pixel).ok_or("Unsupported image type")?;
        let img =
            match color {
                image::ColorType::L16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_luma16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                image::ColorType::Rgb16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                image::ColorType::Rgba16 => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgba<u16>, Vec<u16>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgba16()
                        .ok_or("Could not create image L16 image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                image::ColorType::La16 => {
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
        let meta = value.meta;
        let imgdata = value.imgdata;
        let width = value.width;
        let height = value.height;
        let pixel = value.pixel;
        let color = color_from_usize(pixel).ok_or("Unsupported image type")?;
        let img =
            match color {
                image::ColorType::Rgb32F => {
                    let mut img = DynamicImage::from(
                        ImageBuffer::<image::Rgb<f32>, Vec<f32>>::new(width as u32, height as u32),
                    );
                    let imgbuf = img
                        .as_mut_rgb32f()
                        .ok_or("Could not create image Rgb32F image")?;
                    imgbuf.copy_from_slice(&imgdata);
                    img
                }
                image::ColorType::Rgba32F => {
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
