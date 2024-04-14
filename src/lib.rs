#![deny(missing_docs)]
/*!

# cameraunit

`cameraunit` provides a well-defined and ergonomic API to write interfaces to capture frames from CCD/CMOS based
detectors through Rust traits `cameraunit::CameraUnit` and `cameraunit::CameraInfo`. The library additionally
provides the `cameraunit::ImageData` struct to obtain images with extensive metadata.

You can use `cameraunit` to:
 - Write user-friendly interfaces to C APIs to access different kinds of cameras in a uniform fashion,
 - Acquire images from these cameras in different pixel formats (using the [`image`](https://crates.io/crates/image) crate as a backend),
 - Save these images to `FITS` files (requires the `cfitsio` C library, and uses the [`fitsio`](https://crates.io/crates/fitsio) crate) with extensive metadata,
 - Alternatively, use the internal [`serialimage::DynamicSerialImage`](https://docs.rs/crate/serialimage/latest/) object to obtain `JPEG`, `PNG`, `BMP` etc.

## Usage
Add this to your `Cargo.toml`:
```toml
[dependencies]
cameraunit = "6.0"
```
and this to your source code:
```no_run
use cameraunit::{CameraDriver, CameraUnit, CameraInfo, DynamicSerialImage, OptimumExposureBuilder, SerialImageBuffer};
```

## Example
Since this library is mostly trait-only, refer to projects (such as [`cameraunit_asi`](https://crates.io/crates/cameraunit_asi)) to see it in action.

## Notes
The interface provides two traits:
 1. `CameraUnit`: This trait supports extensive access to the camera, and provides the API for mutating the camera
 state, such as changing the exposure, region of interest on the detector, etc. The object implementing this trait
 should not derive from the `Clone` trait, since ideally image capture should happen in a single thread.
 2. `CameraInfo`: This trait supports limited access to the camera, and provides the API for obtaining housekeeping
 data such as temperatures, gain etc., while allowing limited mutation of the camera state, such as changing the
 detector temperature set point, turning cooler on and off, etc.

Ideally, the crate implementing the camera interface should
 1. Implement the `CameraUnit` and `CameraInfo` for a `struct` that does not allow cloning, and implement a second,
 smaller structure that allows clone and implement only `CameraInfo` for that struct.
 2. Provide functions to get the number of available cameras, a form of unique identification for the cameras,
 and to open a camera using the unique identification. Additionally, a function to open the first available camera
 may be provided.
 3. Upon opening a camera successfully, a tuple of two objects - one implementing the `CameraUnit` trait and
 another implementing the `CameraInfo` trait, should be returned. The second object should be clonable to be
 handed off to some threads if required to handle housekeeping functions.

*/

use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use std::{fmt::Display, time::Duration};
use thiserror::Error;

pub use serialimage::{
    DynamicSerialImage, ImageMetaData, ImageResult, OptimumExposure, OptimumExposureBuilder,
    Primitive, SerialImageBuffer,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Hash)]
/// This structure defines a region of interest.
/// The region of interest is defined in the un-binned pixel space.
pub struct ROI {
    /// The minimum X coordinate (in binned pixel space).
    pub x_min: u32,
    /// The minimum Y coordinate (in binned pixel space).
    pub y_min: u32,
    /// The image width (X axis, in binned pixel space).
    pub width: u32,
    /// The image height (Y axis, in binned pixel space).
    pub height: u32,
    /// The X binning factor.
    pub bin_x: u32,
    /// The Y binning factor.
    pub bin_y: u32,
}

impl Display for ROI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ROI: Origin = ({}, {}), Image Size = ({} x {}), Bin = ({}, {})",
            self.x_min, self.y_min, self.width, self.height, self.bin_x, self.bin_y
        )
    }
}

/// A trait object for a camera unit.
pub type AnyCameraUnit = Box<dyn CameraUnit>;
/// A trait object for a camera info.
pub type AnyCameraInfo = Arc<Box<dyn CameraInfo>>;

/// Trait for camera drivers. Provides functions to
/// list available devices and connect to a device.
#[must_use]
pub trait CameraDriver {
    /// Get the number of available devices.
    fn available_devices(&self) -> usize;
    /// List available devices.
    fn list_devices(&mut self) -> Result<Vec<CameraDescriptor>, Error>;
    /// Connect to a device.
    fn connect_device(
        &mut self,
        descriptor: &CameraDescriptor,
    ) -> Result<(AnyCameraUnit, AnyCameraInfo), Error>;
    /// Connect to the first available device.
    fn connect_first_device(&mut self) -> Result<(AnyCameraUnit, AnyCameraInfo), Error>;
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
/// A structure to hold information about a camera device.
pub struct CameraDescriptor {
    /// The camera ID.
    pub id: usize,
    /// The camera name.
    pub name: String,
}

/// Trait for obtaining camera information and cancelling any ongoing image capture.
/// This trait is intended to be exclusively applied to a clonable object that can
/// be passed to other threads for housekeeping purposes.
#[must_use]
pub trait CameraInfo: Send + Sync {
    /// Check if camera is ready.
    fn camera_ready(&self) -> bool;

    /// Get the camera name.
    fn camera_name(&self) -> &str;

    /// Cancel an ongoing exposure.
    fn cancel_capture(&self) -> Result<(), Error>;

    /// Get any associated unique identifier for the camera.
    ///
    /// Defaults to `None` if unimplemented.
    fn get_uuid(&self) -> Option<&str> {
        None
    }

    /// Check if the camera is currently capturing an image.
    fn is_capturing(&self) -> bool;

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
    fn get_ccd_width(&self) -> u32;

    /// Get the detector height in pixels.
    fn get_ccd_height(&self) -> u32;

    /// Get the detector pixel size (x, y) in microns.
    ///
    /// Defaults to `None` if unimplemented.
    fn get_pixel_size(&self) -> Option<(f32, f32)> {
        None
    }
}

/// Trait for controlling the camera. This trait is intended to be applied to a
/// non-clonable object that is used to capture images and can not be shared across
/// threads.
#[must_use]
pub trait CameraUnit: Send {
    /// Get the camera vendor.
    fn get_vendor(&self) -> &str;

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
    fn capture_image(&self) -> Result<DynamicSerialImage, Error>;

    /// Start an exposure and return. This function does NOT block.
    fn start_exposure(&self) -> Result<(), Error>;

    /// Download the image captured in [`CameraUnit::start_exposure`].
    fn download_image(&self) -> Result<DynamicSerialImage, Error>;

    /// Get exposure status. This function is useful for checking if a
    /// non-blocking exposure has finished running.
    fn image_ready(&self) -> Result<bool, Error>;

    /// Set the exposure time.
    ///
    /// # Arguments
    /// - `exposure` - The exposure time as a [`Duration`].
    ///
    /// # Returns
    /// The exposure time that was set, or error.
    fn set_exposure(&mut self, _exposure: Duration) -> Result<Duration, Error>;

    /// Get the currently set exposure time.
    ///
    /// # Returns
    /// - The exposure time as a [`Duration`].
    fn get_exposure(&self) -> Duration;

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

    /// Set the image region of interest (ROI).
    ///
    /// # Arguments
    /// - `roi` - The region of interest.
    ///
    /// Note:
    /// - The region of interest is defined in the binned pixel space.
    /// - Setting all values to `0` will set the ROI to the full detector size.
    ///
    ///
    /// # Returns
    /// The region of interest that was set, or error.
    fn set_roi(&mut self, roi: &ROI) -> Result<&ROI, Error>;

    /// Set the pixel format.
    ///
    /// # Arguments
    /// - `format` - The pixel format.
    fn set_bpp(&mut self, bpp: PixelBpp) -> Result<PixelBpp, Error>;

    /// Get the pixel format.
    ///
    /// # Returns
    /// The pixel format.
    fn get_bpp(&self) -> PixelBpp;

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
    fn get_bin_x(&self) -> u32 {
        1
    }

    /// Get the Y binning factor.
    ///
    /// Defaults to `1` if unimplemented.
    fn get_bin_y(&self) -> u32 {
        1
    }

    /// Get the region of interest.
    ///
    /// # Returns
    /// - The region of interest.
    fn get_roi(&self) -> &ROI;

    /// Get the current operational status of the camera.
    ///
    /// Defaults to `"Not implemented"` if unimplemented.
    fn get_status(&self) -> String {
        "Not implemented".to_string()
    }

    /// Check if camera is ready.
    fn camera_ready(&self) -> bool;

    /// Get the camera name.
    fn camera_name(&self) -> &str;

    /// Cancel an ongoing exposure.
    fn cancel_capture(&self) -> Result<(), Error>;

    /// Get any associated unique identifier for the camera.
    ///
    /// Defaults to `None` if unimplemented.
    fn get_uuid(&self) -> Option<&str> {
        None
    }

    /// Check if the camera is currently capturing an image.
    fn is_capturing(&self) -> bool;

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
    fn get_ccd_width(&self) -> u32;

    /// Get the detector height in pixels.
    fn get_ccd_height(&self) -> u32;

    /// Get the detector pixel size (x, y) in microns.
    ///
    /// Defaults to `None` if unimplemented.
    fn get_pixel_size(&self) -> Option<(f32, f32)> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Pixel bit depth.
pub enum PixelBpp {
    /// 8 bits per pixel. This is the default.
    Bpp8 = 8,
    /// 10 bits per pixel.
    Bpp10 = 10,
    /// 12 bits per pixel.
    Bpp12 = 12,
    /// 14 bits per pixel.
    Bpp16 = 16,
    /// 16 bits per pixel.
    Bpp24 = 24,
    /// 32 bits per pixel.
    Bpp32 = 32,
}

impl From<u32> for PixelBpp {
    /// Convert from `u32` to [`cameraunit::PixelBpp`].
    ///
    /// # Arguments
    /// - `value` - The value to convert.
    /// Note: If the value is not one of the known values, `Bpp8` is returned.
    ///
    /// # Returns
    /// The corresponding [`cameraunit::PixelBpp`] value.
    fn from(value: u32) -> Self {
        match value {
            8 => PixelBpp::Bpp8,
            10 => PixelBpp::Bpp10,
            12 => PixelBpp::Bpp12,
            16 => PixelBpp::Bpp16,
            24 => PixelBpp::Bpp24,
            32 => PixelBpp::Bpp32,
            _ => PixelBpp::Bpp8,
        }
    }
}

#[derive(Error, Debug, PartialEq, Serialize, Deserialize)]
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
