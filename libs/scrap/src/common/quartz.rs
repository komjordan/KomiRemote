use crate::{quartz, Frame, Pixfmt};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, TryLockError};
use std::{io, mem};

pub struct Capturer {
    inner: quartz::Capturer,
    frame: Arc<Mutex<Option<quartz::Frame>>>,
    saved_raw_data: Vec<u8>, // for faster compare and copy
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        let frame = Arc::new(Mutex::new(None));

        let f = frame.clone();
        let inner = quartz::Capturer::new(
            display.0,
            display.width(),
            display.height(),
            quartz::PixelFormat::Argb8888,
            Default::default(),
            move |inner| {
                if let Ok(mut f) = f.lock() {
                    *f = Some(inner);
                }
            },
        )
        .map_err(|_| io::Error::from(io::ErrorKind::Other))?;

        Ok(Capturer {
            inner,
            frame,
            saved_raw_data: Vec::new(),
        })
    }

    pub fn width(&self) -> usize {
        self.inner.width()
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }
}

impl crate::TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, _timeout_ms: std::time::Duration) -> io::Result<Frame<'a>> {
        match self.frame.try_lock() {
            Ok(mut handle) => {
                let mut frame = None;
                mem::swap(&mut frame, &mut handle);

                match frame {
                    Some(mut frame) => {
                        crate::would_block_if_equal(&mut self.saved_raw_data, frame.inner())?;
                        frame.surface_to_bgra(self.height());
                        Ok(Frame::PixelBuffer(PixelBuffer {
                            frame: PixelStorage::Surface(frame),
                            data: PhantomData,
                            width: self.width(),
                            height: self.height(),
                        }))
                    }

                    None => Err(io::ErrorKind::WouldBlock.into()),
                }
            }

            Err(TryLockError::WouldBlock) => Err(io::ErrorKind::WouldBlock.into()),

            Err(TryLockError::Poisoned(..)) => Err(io::ErrorKind::Other.into()),
        }
    }
}

enum PixelStorage<'a> {
    Surface(quartz::Frame),
    Camera(&'a [u8]),
}

pub struct PixelBuffer<'a> {
    frame: PixelStorage<'a>,
    data: PhantomData<&'a [u8]>,
    width: usize,
    height: usize,
}

impl<'a> PixelBuffer<'a> {
    /// A camera frame borrows tightly packed RGBA bytes owned by its capturer.
    /// It does not allocate or claim ownership of a screen IOSurface.
    pub fn from_camera_rgba(bytes: &'a [u8], width: usize, height: usize) -> io::Result<Self> {
        let expected = width.checked_mul(height).and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(bytes.len()) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid camera RGBA frame"));
        }
        Ok(Self { frame: PixelStorage::Camera(bytes), data: PhantomData, width, height })
    }
}

impl<'a> crate::TraitPixelBuffer for PixelBuffer<'a> {
    fn data(&self) -> &[u8] {
        match &self.frame {
            PixelStorage::Surface(frame) => &*frame,
            PixelStorage::Camera(bytes) => bytes,
        }
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn stride(&self) -> Vec<usize> {
        let mut v = Vec::new();
        v.push(match &self.frame {
            PixelStorage::Surface(frame) => frame.stride(),
            PixelStorage::Camera(_) => self.width * 4,
        });
        v
    }

    fn pixfmt(&self) -> Pixfmt {
        match &self.frame {
            PixelStorage::Surface(_) => Pixfmt::BGRA,
            PixelStorage::Camera(_) => Pixfmt::RGBA,
        }
    }
}

pub struct Display(quartz::Display);

impl Display {
    pub fn primary() -> io::Result<Display> {
        Ok(Display(quartz::Display::primary()))
    }

    pub fn all() -> io::Result<Vec<Display>> {
        Ok(quartz::Display::online()
            .map_err(|_| io::Error::from(io::ErrorKind::Other))?
            .into_iter()
            .map(Display)
            .collect())
    }

    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn height(&self) -> usize {
        self.0.height()
    }

    pub fn scale(&self) -> f64 {
        self.0.scale()
    }

    pub fn name(&self) -> String {
        self.0.id().to_string()
    }

    pub fn is_online(&self) -> bool {
        self.0.is_online()
    }

    pub fn origin(&self) -> (i32, i32) {
        let o = self.0.bounds().origin;
        (o.x as _, o.y as _)
    }

    pub fn is_primary(&self) -> bool {
        self.0.is_primary()
    }
}
