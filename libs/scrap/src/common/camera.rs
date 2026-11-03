use std::{
    io,
    sync::{Arc, Mutex},
};

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use nokhwa::{
    pixel_format::RgbAFormat,
    query,
    utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};

use base::message_proto::{DisplayInfo, Resolution};

#[cfg(feature = "vram")]
use crate::AdapterDevice;

use crate::common::{bail, ResultType};
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::PixelBuffer;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::Pixfmt;
use crate::{Frame, TraitCapturer};

pub const PRIMARY_CAMERA_IDX: usize = 0;
lazy_static::lazy_static! {
    static ref SYNC_CAMERA_DISPLAYS: Arc<Mutex<Vec<DisplayInfo>>> = Arc::new(Mutex::new(Vec::new()));
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const CAMERA_NOT_SUPPORTED: &str = "This platform doesn't support camera yet";

pub struct Cameras;

// pre-condition
pub fn primary_camera_exists() -> bool {
    Cameras::exists(PRIMARY_CAMERA_IDX)
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl Cameras {
    /// Enumerate identifiers and labels without opening devices or locking caches.
    pub fn input_devices() -> ResultType<Vec<(String, String)>> {
        Ok(query(ApiBackend::Auto)?
            .iter()
            .map(|info| {
                // AVFoundation discovery returns ordinal indexes; `misc` contains
                // AVCaptureDevice.uniqueID. Open by that ID, not a reorderable slot.
                #[cfg(target_os = "macos")]
                {
                    return (format!("string:{}", info.misc()), info.human_name().clone());
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let id = match info.index() {
                        CameraIndex::Index(index) => format!("index:{index}"),
                        CameraIndex::String(index) => format!("string:{index}"),
                    };
                    (id, info.human_name().clone())
                }
            })
            .collect())
    }

    pub fn get_input_capturer(id: &str) -> ResultType<Box<dyn TraitCapturer>> {
        let index = if let Some(index) = id.strip_prefix("index:") {
            CameraIndex::Index(index.parse::<u32>()?)
        } else if let Some(index) = id.strip_prefix("string:") {
            if index.is_empty() {
                bail!("Invalid camera input identifier");
            }
            CameraIndex::String(index.to_owned())
        } else {
            bail!("Invalid camera input identifier");
        };
        Ok(Box::new(CameraCapturer::from_index(index)?))
    }
    pub fn all_info() -> ResultType<Vec<DisplayInfo>> {
        match query(ApiBackend::Auto) {
            Ok(cameras) => {
                let mut camera_displays = SYNC_CAMERA_DISPLAYS.lock().unwrap();
                camera_displays.clear();
                // FIXME: nokhwa returns duplicate info for one physical camera on linux for now.
                // issue: https://github.com/l1npengtul/nokhwa/issues/171
                // Use only one camera as a temporary hack.
                cfg_if::cfg_if! {
                    if #[cfg(target_os = "linux")] {
                        let Some(info) = cameras.first() else {
                            bail!("No camera found")
                        };
                        // Use index (0) camera as main camera, fallback to the first camera if index (0) is not available.
                        // But maybe we also need to check index (1) or the lowest index camera.
                        //
                        // https://askubuntu.com/questions/234362/how-to-fix-this-problem-where-sometimes-dev-video0-becomes-automatically-dev
                        // https://github.com/rustdesk/rustdesk/pull/12010#issue-3125329069
                        let mut camera_index = info.index().clone();
                        if !matches!(camera_index, CameraIndex::Index(0)) {
                            if cameras.iter().any(|cam| matches!(cam.index(), CameraIndex::Index(0))) {
                                camera_index = CameraIndex::Index(0);
                            }
                        }
                        let camera = Self::create_camera(&camera_index)?;
                        let resolution = camera.resolution();
                        let (width, height) = (resolution.width() as i32, resolution.height() as i32);
                        camera_displays.push(DisplayInfo {
                            x: 0,
                            y: 0,
                            name: info.human_name().clone(),
                            width,
                            height,
                            online: true,
                            cursor_embedded: false,
                            scale:1.0,
                            original_resolution: Some(Resolution {
                                width,
                                height,
                                ..Default::default()
                            }).into(),
                            ..Default::default()
                        });
                    } else {
                        let mut x = 0;
                        for info in &cameras {
                            let camera = Self::create_camera(info.index())?;
                            let resolution = camera.resolution();
                            let (width, height) = (resolution.width() as i32, resolution.height() as i32);
                            camera_displays.push(DisplayInfo {
                                x,
                                y: 0,
                                name: info.human_name().clone(),
                                width,
                                height,
                                online: true,
                                cursor_embedded: false,
                                scale:1.0,
                                original_resolution: Some(Resolution {
                                    width,
                                    height,
                                    ..Default::default()
                                }).into(),
                                ..Default::default()
                            });
                            x += width;
                        }
                    }
                }
                Ok(camera_displays.clone())
            }
            Err(e) => {
                bail!("Query cameras error: {}", e)
            }
        }
    }

    pub fn exists(index: usize) -> bool {
        match query(ApiBackend::Auto) {
            Ok(cameras) => index < cameras.len(),
            _ => return false,
        }
    }

    fn create_camera(index: &CameraIndex) -> ResultType<Camera> {
        // Permission is requested explicitly by the GUI on the main thread.
        // Never prompt from a capture worker, discovery, or a privileged helper.
        #[cfg(target_os = "macos")]
        if !nokhwa::nokhwa_check() {
            bail!("Camera permission required: authorize camera access in the local KomiRemote window");
        }
        let format_type = if cfg!(target_os = "linux") {
            RequestedFormatType::None
        } else {
            RequestedFormatType::AbsoluteHighestResolution
        };
        let result = Camera::new(
            index.clone(),
            RequestedFormat::new::<RgbAFormat>(format_type),
        );
        match result {
            Ok(camera) => Ok(camera),
            Err(e) => bail!("create camera{} error:  {}", index, e),
        }
    }

    pub fn get_camera_resolution(index: usize) -> ResultType<Resolution> {
        let index = Self::index_for_position(index)?;
        let camera = Self::create_camera(&index)?;
        let resolution = camera.resolution();
        Ok(Resolution {
            width: resolution.width() as i32,
            height: resolution.height() as i32,
            ..Default::default()
        })
    }

    pub fn get_sync_cameras() -> Vec<DisplayInfo> {
        SYNC_CAMERA_DISPLAYS.lock().unwrap().clone()
    }

    pub fn get_capturer(current: usize) -> ResultType<Box<dyn TraitCapturer>> {
        Ok(Box::new(CameraCapturer::new(current)?))
    }

    fn index_for_position(current: usize) -> ResultType<CameraIndex> {
        #[cfg(target_os = "macos")]
        {
            let devices = query(ApiBackend::Auto)?;
            let info = devices
                .get(current)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Camera disconnected"))?;
            Ok(CameraIndex::String(info.misc().clone()))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(CameraIndex::Index({
                if current > u32::MAX as usize {
                    bail!("Invalid camera index");
                }
                current as u32
            }))
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
impl Cameras {
    pub fn input_devices() -> ResultType<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    pub fn get_input_capturer(_id: &str) -> ResultType<Box<dyn TraitCapturer>> {
        bail!(CAMERA_NOT_SUPPORTED);
    }

    pub fn all_info() -> ResultType<Vec<DisplayInfo>> {
        return Ok(Vec::new());
    }

    pub fn exists(_index: usize) -> bool {
        false
    }

    pub fn get_camera_resolution(_index: usize) -> ResultType<Resolution> {
        bail!(CAMERA_NOT_SUPPORTED);
    }

    pub fn get_sync_cameras() -> Vec<DisplayInfo> {
        vec![]
    }

    pub fn get_capturer(_current: usize) -> ResultType<Box<dyn TraitCapturer>> {
        bail!(CAMERA_NOT_SUPPORTED);
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub struct CameraCapturer {
    camera: Camera,
    data: Vec<u8>,
    last_data: Vec<u8>, // for faster compare and copy
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub struct CameraCapturer;

impl CameraCapturer {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn new(current: usize) -> ResultType<Self> {
        Self::from_index(Cameras::index_for_position(current)?)
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn from_index(index: CameraIndex) -> ResultType<Self> {
        let camera = Cameras::create_camera(&index)?;
        Ok(CameraCapturer {
            camera,
            data: Vec::new(),
            last_data: Vec::new(),
        })
    }

    #[allow(dead_code)]
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn new(_current: usize) -> ResultType<Self> {
        bail!(CAMERA_NOT_SUPPORTED);
    }
}

impl TraitCapturer for CameraCapturer {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn frame<'a>(&'a mut self, _timeout: std::time::Duration) -> std::io::Result<Frame<'a>> {
        #[cfg(target_os = "macos")]
        if !nokhwa::nokhwa_check() {
            let _ = self.camera.stop_stream();
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Camera access is not authorized",
            ));
        }
        if !self.camera.is_stream_open() {
            if let Err(e) = self.camera.open_stream() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Camera open stream error: {}", e),
                ));
            }
        }
        match self.camera.frame() {
            Ok(buffer) => {
                match buffer.decode_image::<RgbAFormat>() {
                    Ok(decoded) => {
                        self.data = decoded.as_raw().to_vec();
                        crate::would_block_if_equal(&mut self.last_data, &self.data)?;
                        // Decoder output is tightly packed RGBA, not a Quartz IOSurface.
                        cfg_if::cfg_if! {
                            if #[cfg(any(target_os = "linux", target_os = "windows"))] {
                                Ok(Frame::PixelBuffer(PixelBuffer::new(
                                    &self.data,
                                    Pixfmt::RGBA,
                                    decoded.width() as usize,
                                    decoded.height() as usize,
                                )))
                            } else if #[cfg(target_os = "macos")] {
                                Ok(Frame::PixelBuffer(PixelBuffer::from_camera_rgba(
                                    &self.data,
                                    decoded.width() as usize,
                                    decoded.height() as usize,
                                )?))
                            }

                        }
                    }
                    Err(e) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Camera frame decode error: {}", e),
                    )),
                }
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Camera frame error: {}", e),
            )),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn frame<'a>(&'a mut self, _timeout: std::time::Duration) -> std::io::Result<Frame<'a>> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            CAMERA_NOT_SUPPORTED.to_string(),
        ))
    }

    #[cfg(windows)]
    fn is_gdi(&self) -> bool {
        true
    }

    #[cfg(windows)]
    fn set_gdi(&mut self) -> bool {
        true
    }

    #[cfg(feature = "vram")]
    fn device(&self) -> AdapterDevice {
        AdapterDevice::default()
    }

    #[cfg(feature = "vram")]
    fn set_output_texture(&mut self, _texture: bool) {}
}
