use std::any::Any;
use std::{fmt::Display, time::Duration};
use thiserror::Error;

mod imagedata;
pub use imagedata::ImageData; 
pub use serialimagedata::{ImageMetaData, SerialImageData, SerialImagePixel, SerialImageStorageTypes};
pub use image::{DynamicImage, ImageBuffer, ColorType, Pixel};

#[deny(missing_docs)]
#[derive(Clone, Copy)]
/// This structure defines a region of interest.
/// The region of interest is defined in the un-binned pixel space.
pub struct ROI {
    /// The minimum X coordinate (in unbinned pixel space).
    pub x_min: i32,
    /// The maximum X coordinate (in unbinned pixel space).
    pub x_max: i32,
    /// The minimum Y coordinate (in unbinned pixel space).
    pub y_min: i32,
    /// The maximum Y coordinate (in unbinned pixel space).
    pub y_max: i32,
    /// The X binning factor.
    pub bin_x: i32,
    /// The Y binning factor.
    pub bin_y: i32,
}

impl Display for ROI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(ROI: x_min = {}, x_max = {}, y_min: {}, y_max = {}, bin_x = {}, bin_y = {})",
            self.x_min, self.x_max, self.y_min, self.y_max, self.bin_x, self.bin_y
        )
    }
}

/// Trait for obtaining camera information and cancelling any ongoing image capture.
/// This trait is intended to be exclusively applied to a clonable object that can
/// be passed to other threads for housekeeping purposes.
pub trait CameraInfo {
    /// Check if camera is ready.
    /// 
    /// Defaults to `false` if unimplemented.
    fn camera_ready(&self) -> bool {
        false
    }

    /// Get the camera name.
    /// 
    /// Defaults to `"Unknown"` if unimplemented.
    fn camera_name(&self) -> &str {
        "Unknown"
    }

    /// Cancel an ongoing exposure.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn cancel_capture(&self) -> Result<(), Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get any associated unique identifier for the camera.
    /// 
    /// Defaults to `None` if unimplemented.
    fn get_uuid(&self) -> Option<&str> {
        None
    }

    /// Check if the camera is currently capturing an image.
    /// 
    /// Defaults to `false` if unimplemented.
    fn is_capturing(&self) -> bool {
        false
    }

    /// Set the target detector temperature.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_temperature(&self, _temperature: f32) -> Result<f32, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the current detector temperature.
    /// 
    /// Defaults to `None` if unimplemented.
    fn get_temperature(&self) -> Option<f32> {
        None
    }

    /// Enable/disable cooler.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_cooler(&self, _on: bool) -> Result<(), Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Check if cooler is enabled/disabled.
    /// 
    /// Defaults to `None` if unimplemented/not available.
    fn get_cooler(&self) -> Option<bool> {
        None
    }

    /// Get the current cooler power.
    /// 
    /// Defaults to `None` if unimplemented.
    fn get_cooler_power(&self) -> Option<f32> {
        None
    }

    /// Set the cooler power.
    /// 
    /// Raises a `GeneralError` with the message `"Not implemented"` if unimplemented.
    fn set_cooler_power(&self, _power: f32) -> Result<f32, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the detector width in pixels.
    /// 
    /// Defaults to `0` if unimplemented.
    fn get_ccd_width(&self) -> u32 {
        0
    }

    /// Get the detector height in pixels.
    /// 
    /// Defaults to `0` if unimplemented.
    fn get_ccd_height(&self) -> u32 {
        0
    }

    /// Get the detector pixel size in microns.
    /// 
    /// Defaults to `None` if unimplemented.
    fn get_pixel_size(&self) -> Option<f32> {
        None
    }
}

pub trait CameraUnit : CameraInfo {
    /// Get the camera vendor.
    /// 
    /// Defaults to `"Unknown"` if unimplemented.
    fn get_vendor(&self) -> &str {
        "Unknown"
    }

    /// Get a handle to the internal camera. This is intended to be used for
    /// development purposes, as (presumably FFI and unsafe) internal calls
    /// are abstracted away from the user.
    /// 
    /// Defaults to `None` if unimplemented.
    fn get_handle(&self) -> Option<&dyn Any> {
        None
    }

    /// Capture an image.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn capture_image(&self) -> Result<ImageData, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Start an exposure and return. This function does NOT block.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn start_exposure(&self) -> Result<(), Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Download the image captured in [`CameraUnit::start_exposure()`].
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn download_image(&self) -> Result<ImageData, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get exposure status. This function is useful for checking if a
    /// non-blocking exposure has finished running.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn image_ready(&self) -> Result<bool, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Set the exposure time.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_exposure(&mut self, _exposure: Duration) -> Result<Duration, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the currently set exposure time.
    /// 
    /// Defaults to `Duration::from_secs(0)` if unimplemented.
    fn get_exposure(&self) -> Duration {
        Duration::from_secs(0)
    }

    /// Get the current gain (in percentage units).
    /// 
    /// Defaults to `0.0` if unimplemented.
    fn get_gain(&self) -> f32 {
        0.0
    }

    /// Get the current gain (in raw values).
    /// 
    /// Defaults to `0` if unimplemented.
    fn get_gain_raw(&self) -> i64 {
        0
    }

    /// Set the gain (in percentage units).
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_gain(&mut self, _gain: f32) -> Result<f32, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Set the gain (in raw values).
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_gain_raw(&mut self, _gain: i64) -> Result<i64, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the current pixel offset.
    /// 
    /// Defaults to `0` if unimplemented.
    fn get_offset(&self) -> i32 {
        0
    }

    /// Set the pixel offset.
    /// 
    /// Raises a `GeneralError` with the message `"Not implemented"` if unimplemented.
    fn set_offset(&mut self, _offset: i32) -> Result<i32, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the minimum exposure time.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn get_min_exposure(&self) -> Result<Duration, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the maximum exposure time.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn get_max_exposure(&self) -> Result<Duration, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the minimum gain (in raw units).
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn get_min_gain(&self) -> Result<i64, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the maximum gain (in raw units).
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn get_max_gain(&self) -> Result<i64, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Set the shutter to open (always/when exposing).
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_shutter_open(&mut self, _open: bool) -> Result<bool, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Get the shutter state.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn get_shutter_open(&self) -> Result<bool, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Set the region of interest and binning.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_roi(&mut self, _roi: &ROI) -> Result<&ROI, Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Flip the image along X and/or Y axes.
    /// 
    /// Raises a `Message` with the message `"Not implemented"` if unimplemented.
    fn set_flip(&mut self, _x: bool, _y: bool) -> Result<(), Error> {
        Err(Error::Message("Not implemented".to_string()))
    }

    /// Check if the image is flipped along X and/or Y axes.
    /// 
    /// Defaults to `(false, false)` if unimplemented.
    fn get_flip(&self) -> (bool, bool) {
        (false, false)
    }

    /// Get the X binning factor.
    /// 
    /// Defaults to `1` if unimplemented.
    fn get_bin_x(&self) -> i32 {
        1
    }

    /// Get the Y binning factor.
    /// 
    /// Defaults to `1` if unimplemented.
    fn get_bin_y(&self) -> i32 {
        1
    }

    /// Get the region of interest.
    /// 
    /// Defaults to `ROI{x_min: 0, x_max: 0, y_min: 0, y_max: 0, bin_x: 1, bin_y: 1}` if unimplemented.
    fn get_roi(&self) -> &ROI {
        &ROI {
            x_min: 0,
            x_max: 0,
            y_min: 0,
            y_max: 0,
            bin_x: 1,
            bin_y: 1,
        }
    }

    /// Get the current operational status of the camera.
    /// 
    /// Defaults to `"Not implemented"` if unimplemented.
    fn get_status(&self) -> String {
        "Not implemented".to_string()
    }
}

#[derive(Error, Debug, PartialEq)]
/// Errors returned by camera operations.
pub enum Error {
    /// Error message.
    #[error("Error: {0}")]
    Message(String),
    /// Invalid index.
    #[error("Invalid index: {0}")]
    InvalidIndex(i32),
    /// Invalid ID.
    #[error("Invalid ID: {0}")]
    InvalidId(i32),
    /// Invalid control type.
    #[error("Invalid control type: {0}")]
    InvalidControlType(String),
    /// No cameras available.
    #[error("No cameras available")]
    NoCamerasAvailable,
    /// Camera not open for access.
    #[error("Camera not open for access")]
    CameraClosed,
    /// Camera already removed.
    #[error("Camera already removed")]
    CameraRemoved,
    /// Invalid path.
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    /// Invalid format.
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    /// Invalid size.
    #[error("Invalid size: {0}")]
    InvalidSize(usize),
    /// Invalid image type.
    #[error("Invalid image type: {0}")]
    InvalidImageType(String),
    /// Operation timed out.
    #[error("Operation timed out")]
    TimedOut,
    /// Invalid sequence.
    #[error("Invalid sequence")]
    InvalidSequence,
    /// Buffer too small.
    #[error("Buffer too small: {0}")]
    BufferTooSmall(usize),
    /// Exposure in progress.
    #[error("Exposure already in progress")]
    ExposureInProgress,
    /// General error.
    #[error("General error: {0}")]
    GeneralError(String),
    /// Invalid mode.
    #[error("Invalid mode: {0}")]
    InvalidMode(String),
    /// Exposure failed.
    #[error("Exposure failed: {0}")]
    ExposureFailed(String),
    /// Invalid value.
    #[error("Invalid value: {0}")]
    InvalidValue(String),
    /// Out of bounds.
    #[error("Out of bounds: {0}")]
    OutOfBounds(String),
}

#[cfg(test)]
mod tests {
    use std::{path::Path, time::{Duration, UNIX_EPOCH, SystemTime}};

    use crate::SerialImageData;

    use super::*;
    use image::{DynamicImage, ImageBuffer};
    use rand::Rng;

    fn get_timestamp_millis(tstamp: SystemTime) -> u64 {
        tstamp.duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_millis() as u64
    }

    #[test]
    fn test_write_image() {
        let mut img = {
            let mut meta: ImageMetaData = Default::default();
            meta.timestamp = SystemTime::now();
            meta.camera_name = "ZWO ASI533MM Pro".to_string();
            meta.add_extended_attrib("TEST", "TEST");
            let img = DynamicImage::from(ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(800, 600));
            imagedata::ImageData::new(img, meta)
        };
        let bimg = img.get_image_mut().as_mut_luma16().unwrap();
        let mut rng = rand::thread_rng();
        let vals: Vec<u16> = (0..bimg.width() * bimg.height())
            .map(|_| rng.gen_range(0..255 * 255))
            .collect();
        bimg.copy_from_slice(&vals);
        img.save_fits(Path::new("."), "test", "testprog", true, true)
            .unwrap();
        img.get_image().save(format!("test_{}.png", get_timestamp_millis(img.get_metadata().timestamp))).unwrap();
        let simg: SerialImageData<u16> = img.clone().try_into().unwrap();
        let imgstr = serde_json::to_string(&simg).unwrap();
        let simg = serde_json::from_str::<SerialImageData<u16>>(&imgstr).unwrap();
        let res: Result<SerialImageData<u8>, &'static str> = img.try_into();
        assert!(res.is_err());
        let res: Result<ImageData, &str> = simg.try_into();
        assert!(res.is_ok());
        res.unwrap().save_fits(Path::new("."), "testser", "testprog", true, true)
            .unwrap();

        let mut img = {
            let mut meta: ImageMetaData = Default::default();
            meta.timestamp = SystemTime::now();
            meta.camera_name = "ZWO ASI533MM Pro".to_string();
            meta.add_extended_attrib("TEST", "TEST");
            let img = DynamicImage::from(ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(800, 600));
            imagedata::ImageData::new(img, meta)
        };
        let bimg = img.get_image_mut().as_mut_rgb16().unwrap();
        let mut rng = rand::thread_rng();
        let vals: Vec<u16> = (0..bimg.width() * bimg.height() * 3)
            .map(|_| rng.gen_range(0..255 * 255))
            .collect();
        bimg.copy_from_slice(&vals);
        img.save_fits(Path::new("."), "test_color", "testprog", true, true)
            .unwrap();
        img.get_image().save(format!("test_color_{}.png", get_timestamp_millis(img.get_metadata().timestamp))).unwrap();
    }
}