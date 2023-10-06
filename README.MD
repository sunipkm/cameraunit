# cameraunit

`cameraunit` provides a well-defined and ergonomic API to write interfaces to capture frames from CCD/CMOS based
detectors through Rust traits `cameraunit::CameraUnit` and `cameraunit::CameraInfo`. The library additionally
provides the `cameraunit::ImageData` struct to obtain images with extensive metadata.

You can use `cameraunit` to:
 - Write user-friendly interfaces to C APIs to access different kinds of cameras in an uniform fashion,
 - Acquire images from these cameras in different pixel formats (using the [`image`](https://crates.io/crates/image) crate as a backend),
 - Save these images to `FITS` files (requires the `cfitsio` C library, and uses the [`fitsio`](https://crates.io/crates/fitsio) crate) with extensive metadata,
 - Alternatively, use the internal [`image::DynamicImage`](https://docs.rs/image/0.24.7/image/enum.DynamicImage.html) object to obtain `JPEG`, `PNG`, `BMP` etc.

## Usage
Add this to your `Cargo.toml`:
```toml
[dependencies]
cameraunit = "4.0.0"
```
and this to your source code:
```rs
use cameraunit::{CameraUnit, CameraInfo, DynamicSerialImage, OptimumExposureConfig, SerialImageBuffer};
```

## Example
Since this library is mostly trait-only, refer to projects (such as [`cameraunit_asi`](https://crates.io/crates/cameraunit_asi)) to see it in action.

## Notes
The interface provides two traits:
 1. `CameraUnit`: This trait supports extensive access to the camera, and provides the API for mutating the camera
 state, such as changing the exposure, region of interest on the detector, etc. The object implementing this trait
 should not derive from the `Clone` trait, since ideally image capture should happen in a single thread.
 2. `CameraInfo`: This trait supports limited access to the camera, and provides the API for obtaining housekeeping
 data such as temperatures, gain etc while allowing limited mutation of the camera state, such as changing the
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