//! KomiRemote v22: bidirectional video call on an already authenticated control
//! connection.
//!
//! Signaling is a versioned protobuf message (`KomiVideoCall`). Camera pictures
//! travel in `KomiCallCameraFrame`, which nests a `VideoFrame`. They are never
//! sent as `Message.video_frame`, so the existing screen / view-camera pipeline
//! does not treat them as the desktop stream.
//!
//! Local initiative opens preview immediately; ACCEPT opens the recipient's
//! camera. Only Active calls transmit. Terminal states stop both directions.

use base::message_proto::{
    video_frame, EncodedVideoFrame, EncodedVideoFrames, KomiCallCameraFrame, KomiVideoCall,
    Message, VideoFrame,
};
use hbb_common::{log, protobuf::MessageField};
use scrap::TraitPixelBuffer;
use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const VERSION: u32 = 1;
pub const ACTION_CAPABILITY: u32 = 1;
pub const ACTION_REQUEST: u32 = 2;
pub const ACTION_ACCEPT: u32 = 3;
pub const ACTION_REFUSE: u32 = 4;
pub const ACTION_HANGUP: u32 = 5;
pub const ACTION_CAMERA_ON: u32 = 6;
pub const ACTION_CAMERA_OFF: u32 = 7;
pub const ACTION_EXPIRE: u32 = 8;
pub const ACTION_ERROR: u32 = 9;
pub const ACTION_KEYFRAME: u32 = 10;
pub const ACTION_BUSY: u32 = 11;

pub const STREAM_CONTROLLER: u32 = 1;
pub const STREAM_HOST: u32 = 2;

pub const MAX_WIDTH: u32 = 640;
pub const MAX_HEIGHT: u32 = 360;
pub const MAX_FPS: u32 = 12;
pub const MAX_KBPS: u32 = 600;
pub const MAX_FRAME_BYTES: usize = 96 * 1024;
pub const INVITE_MS: i64 = 45_000;

const JPEG_QUALITY: u8 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Outgoing,
    Incoming,
    Active,
    Ended,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Outgoing => "outgoing",
            Phase::Incoming => "incoming",
            Phase::Active => "active",
            Phase::Ended => "ended",
        }
    }

    pub fn in_call(self) -> bool {
        matches!(self, Phase::Active)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    pub call_id: String,
    pub version: u32,
    pub action: u32,
    pub reason: String,
    pub nonce: u32,
    pub expires_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteEffect {
    None,
    Changed,
    Reply(Signal),
}

/// Pure call state. No device, socket or file is touched here.
#[derive(Clone, Debug)]
pub struct Machine {
    phase: Phase,
    call_id: String,
    nonce: u32,
    expires_ms: i64,
    local_camera: bool,
    remote_camera: bool,
    peer_version: u32,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            call_id: String::new(),
            nonce: 0,
            expires_ms: 0,
            local_camera: false,
            remote_camera: false,
            peer_version: 0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn local_camera(&self) -> bool {
        self.local_camera
    }

    pub fn remote_camera(&self) -> bool {
        self.remote_camera
    }

    pub fn request(&mut self, now_ms: i64) -> Result<Signal, &'static str> {
        if !matches!(self.phase, Phase::Idle | Phase::Ended) {
            return Err("busy");
        }
        self.reset_media();
        self.call_id = new_call_id();
        self.nonce = nonce_of(&self.call_id);
        self.expires_ms = now_ms.saturating_add(INVITE_MS);
        self.phase = Phase::Outgoing;
        // Clicking Video is local consent to preview, not consent to transmit.
        self.local_camera = true;
        Ok(self.signal(ACTION_REQUEST, ""))
    }

    /// One-way offer: this side's camera, not a bidirectional video call.
    /// `call_id` is prefixed so both UIs can tell the modes apart.
    pub fn request_show(&mut self, now_ms: i64) -> Result<Signal, &'static str> {
        if !matches!(self.phase, Phase::Idle | Phase::Ended) {
            return Err("busy");
        }
        self.reset_media();
        self.call_id = format!("cam:{}", new_call_id());
        self.nonce = nonce_of(&self.call_id);
        self.expires_ms = now_ms.saturating_add(INVITE_MS);
        self.phase = Phase::Outgoing;
        self.local_camera = true;
        Ok(self.signal(ACTION_REQUEST, "show-camera"))
    }

    pub fn is_show_camera(&self) -> bool {
        self.call_id.starts_with("cam:")
    }

    pub fn accept(&mut self, now_ms: i64) -> Result<Signal, &'static str> {
        if self.phase != Phase::Incoming {
            return Err("no-invite");
        }
        if self.expired(now_ms) {
            self.finish("expired");
            return Err("expired");
        }
        self.phase = Phase::Active;
        // A camera-only offer must not turn the acceptor's camera on.
        self.local_camera = !self.is_show_camera();
        self.remote_camera = false;
        Ok(self.signal(ACTION_ACCEPT, ""))
    }

    pub fn refuse(&mut self) -> Result<Signal, &'static str> {
        if self.phase != Phase::Incoming {
            return Err("no-invite");
        }
        let sig = self.signal(ACTION_REFUSE, "refused");
        self.finish("refused");
        Ok(sig)
    }

    pub fn hangup(&mut self) -> Signal {
        let sig = self.signal(ACTION_HANGUP, "hangup");
        self.finish("hangup");
        sig
    }

    pub fn camera_on(&mut self) -> Result<Signal, &'static str> {
        if self.phase != Phase::Active {
            return Err("not-active");
        }
        self.local_camera = true;
        Ok(self.signal(ACTION_CAMERA_ON, ""))
    }

    pub fn camera_off(&mut self) -> Result<Signal, &'static str> {
        if !self.local_camera {
            return Err("camera-off");
        }
        self.local_camera = false;
        Ok(self.signal(ACTION_CAMERA_OFF, ""))
    }

    pub fn on_remote(&mut self, sig: &Signal, now_ms: i64) -> RemoteEffect {
        if sig.version == 0 || sig.version > VERSION {
            if sig.action == ACTION_REQUEST || sig.action == ACTION_CAPABILITY {
                return RemoteEffect::Reply(self.error_for(sig, "unsupported"));
            }
            return RemoteEffect::None;
        }
        self.peer_version = sig.version;
        match sig.action {
            ACTION_CAPABILITY => RemoteEffect::Changed,
            ACTION_REQUEST => self.on_request(sig, now_ms),
            ACTION_ACCEPT => self.on_accept(sig, now_ms),
            ACTION_REFUSE | ACTION_BUSY | ACTION_EXPIRE | ACTION_ERROR => {
                self.on_terminal(sig, "peer")
            }
            ACTION_HANGUP => self.on_terminal(sig, "hangup"),
            ACTION_CAMERA_ON => {
                if self.same_active(sig) {
                    self.remote_camera = true;
                    RemoteEffect::Changed
                } else {
                    RemoteEffect::None
                }
            }
            ACTION_CAMERA_OFF => {
                if self.call_id == sig.call_id {
                    self.remote_camera = false;
                    RemoteEffect::Changed
                } else {
                    RemoteEffect::None
                }
            }
            _ => RemoteEffect::None,
        }
    }

    pub fn tick(&mut self, now_ms: i64) -> Option<Signal> {
        if matches!(self.phase, Phase::Outgoing | Phase::Incoming) && self.expired(now_ms) {
            let sig = self.signal(ACTION_EXPIRE, "expired");
            self.finish("expired");
            Some(sig)
        } else {
            None
        }
    }

    /// Frames are accepted only on the live call, after the peer has turned its
    /// camera on (or with the first frame of an already accepted call, which
    /// implies that consent — the sender must not emit before CAMERA_ON).
    pub fn allow_frame(&mut self, call_id: &str, nbytes: usize) -> bool {
        if self.phase != Phase::Active || call_id.is_empty() || self.call_id != call_id {
            return false;
        }
        if nbytes == 0 || nbytes > MAX_FRAME_BYTES {
            return false;
        }
        self.remote_camera = true;
        true
    }

    fn on_request(&mut self, sig: &Signal, now_ms: i64) -> RemoteEffect {
        if !matches!(self.phase, Phase::Idle | Phase::Ended) {
            return RemoteEffect::Reply(self.busy_for(sig));
        }
        if sig.call_id.is_empty() || sig.nonce == 0 {
            return RemoteEffect::Reply(self.error_for(sig, "bad-call"));
        }
        self.reset_media();
        self.call_id = sig.call_id.clone();
        self.nonce = sig.nonce;
        self.expires_ms = if sig.expires_ms > now_ms {
            sig.expires_ms.min(now_ms.saturating_add(INVITE_MS))
        } else {
            now_ms.saturating_add(INVITE_MS)
        };
        self.phase = Phase::Incoming;
        RemoteEffect::Changed
    }

    fn on_accept(&mut self, sig: &Signal, now_ms: i64) -> RemoteEffect {
        if self.phase != Phase::Outgoing || self.call_id != sig.call_id || self.nonce != sig.nonce {
            return RemoteEffect::None;
        }
        if self.expired(now_ms) {
            self.finish("expired");
            return RemoteEffect::Changed;
        }
        self.phase = Phase::Active;
        // Keep the requester's local choice (including preview switched off).
        self.remote_camera = false;
        RemoteEffect::Changed
    }

    fn on_terminal(&mut self, sig: &Signal, reason: &str) -> RemoteEffect {
        if self.call_id != sig.call_id || matches!(self.phase, Phase::Idle) {
            return RemoteEffect::None;
        }
        self.finish(reason);
        RemoteEffect::Changed
    }

    fn same_active(&self, sig: &Signal) -> bool {
        self.phase == Phase::Active && self.call_id == sig.call_id
    }

    fn expired(&self, now_ms: i64) -> bool {
        self.expires_ms > 0 && now_ms > self.expires_ms
    }

    fn finish(&mut self, _reason: &str) {
        self.phase = Phase::Ended;
        self.reset_media();
    }

    fn reset_media(&mut self) {
        self.local_camera = false;
        self.remote_camera = false;
    }

    fn signal(&self, action: u32, reason: &str) -> Signal {
        Signal {
            call_id: self.call_id.clone(),
            version: VERSION,
            action,
            reason: reason.to_owned(),
            nonce: self.nonce,
            expires_ms: self.expires_ms,
        }
    }

    fn busy_for(&self, sig: &Signal) -> Signal {
        Signal {
            call_id: sig.call_id.clone(),
            version: VERSION,
            action: ACTION_BUSY,
            reason: "busy".to_owned(),
            nonce: sig.nonce,
            expires_ms: 0,
        }
    }

    fn error_for(&self, sig: &Signal, reason: &str) -> Signal {
        Signal {
            call_id: sig.call_id.clone(),
            version: VERSION,
            action: ACTION_ERROR,
            reason: reason.to_owned(),
            nonce: sig.nonce,
            expires_ms: 0,
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_call_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn nonce_of(call_id: &str) -> u32 {
    let bytes = call_id.as_bytes();
    let mut n = 0x9E3779B9u32;
    for (i, b) in bytes.iter().enumerate() {
        n = n
            .wrapping_mul(16777619)
            .wrapping_add(*b as u32)
            .wrapping_add(i as u32);
    }
    if n == 0 {
        1
    } else {
        n
    }
}

pub fn signal_to_message(sig: &Signal) -> Message {
    let mut call = KomiVideoCall::new();
    call.call_id = sig.call_id.clone();
    call.version = sig.version;
    call.action = sig.action;
    call.reason = sig.reason.clone();
    call.nonce = sig.nonce;
    call.expires_unix_ms = sig.expires_ms;
    call.max_width = MAX_WIDTH;
    call.max_height = MAX_HEIGHT;
    call.max_fps = MAX_FPS;
    call.max_kbps = MAX_KBPS;
    let mut msg = Message::new();
    msg.set_komi_video_call(call);
    msg
}

pub fn message_to_signal(call: &KomiVideoCall) -> Signal {
    Signal {
        call_id: call.call_id.clone(),
        version: call.version,
        action: call.action,
        reason: call.reason.clone(),
        nonce: call.nonce,
        expires_ms: call.expires_unix_ms,
    }
}

pub fn frame_message(call_id: &str, stream_id: u32, seq: u32, frame: VideoFrame) -> Message {
    let mut wrap = KomiCallCameraFrame::new();
    wrap.call_id = call_id.to_owned();
    wrap.stream_id = stream_id;
    wrap.seq = seq;
    wrap.timestamp_ms = now_ms();
    wrap.key = frame_is_key(&frame);
    wrap.frame = MessageField::some(frame);
    let mut msg = Message::new();
    msg.set_komi_call_camera_frame(wrap);
    msg
}

pub fn video_frame_len(frame: &VideoFrame) -> usize {
    let Some(union) = frame.union.as_ref() else {
        return 0;
    };
    let frames = match union {
        video_frame::Union::Vp8s(v) => &v.frames,
        video_frame::Union::Vp9s(v) => &v.frames,
        video_frame::Union::H264s(v) => &v.frames,
        video_frame::Union::H265s(v) => &v.frames,
        video_frame::Union::Av1s(v) => &v.frames,
        _ => return 0,
    };
    frames.iter().map(|f| f.data.len()).sum()
}

fn frame_is_key(frame: &VideoFrame) -> bool {
    let Some(union) = frame.union.as_ref() else {
        return false;
    };
    let frames = match union {
        video_frame::Union::Vp8s(v) => &v.frames,
        video_frame::Union::Vp9s(v) => &v.frames,
        _ => return false,
    };
    frames.iter().any(|f| f.key)
}

pub fn fit_even(width: u32, height: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let width = width.max(2);
    let height = height.max(2);
    let max_w = max_w.max(2);
    let max_h = max_h.max(2);
    let scale = (max_w as f32 / width as f32)
        .min(max_h as f32 / height as f32)
        .min(1.0);
    let mut w = ((width as f32 * scale).round() as u32).max(2);
    let mut h = ((height as f32 * scale).round() as u32).max(2);
    w &= !1;
    h &= !1;
    (w.max(2), h.max(2))
}

fn scale_rgba(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut out = vec![0u8; dw * dh * 4];
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 || src.len() < sw * sh * 4 {
        return out;
    }
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            let s = (sy * sw + sx) * 4;
            let d = (y * dw + x) * 4;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}

pub fn rgba_to_jpeg(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() < (width as usize) * (height as usize) * 4 {
        return None;
    }
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    let mut jpeg = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY);
    enc.encode(&rgb, width, height, image::ColorType::Rgb8)
        .ok()?;
    if jpeg.is_empty() || jpeg.len() > MAX_FRAME_BYTES {
        return None;
    }
    Some(jpeg)
}

fn decoded_to_jpeg(rgb: &scrap::ImageRgb) -> Option<Vec<u8>> {
    let w = rgb.w;
    let h = rgb.h;
    if w == 0 || h == 0 || w > 1920 || h > 1080 {
        return None;
    }
    let align = rgb.align.max(1);
    let stride = (w * 4 + align - 1) & !(align - 1);
    if rgb.raw.len() < stride.saturating_mul(h) {
        return None;
    }
    let mut packed = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &rgb.raw[y * stride..y * stride + w * 4];
        for px in row.chunks_exact(4) {
            // libyuv ABGR is R,G,B,A in memory; ARGB is B,G,R,A.
            let (r, g, b, a) = match rgb.fmt {
                scrap::ImageFormat::ABGR => (px[0], px[1], px[2], px[3]),
                scrap::ImageFormat::ARGB => (px[2], px[1], px[0], px[3]),
                scrap::ImageFormat::Raw => (px[0], px[1], px[2], 255),
            };
            packed.extend_from_slice(&[r, g, b, a]);
        }
    }
    rgba_to_jpeg(&packed, w as u32, h as u32)
}

fn preview_path(key: &str, stream_id: u32) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("komi-video-call");
    let _ = std::fs::create_dir_all(&dir);
    let safe: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(48)
        .collect();
    dir.push(format!("{safe}-{stream_id}.jpg"));
    dir
}

pub fn write_preview(key: &str, stream_id: u32, jpeg: &[u8]) -> Option<PathBuf> {
    if jpeg.is_empty() || jpeg.len() > MAX_FRAME_BYTES {
        return None;
    }
    let path = preview_path(key, stream_id);
    let temporary = path.with_extension("jpg.part");
    let mut file = std::fs::File::create(&temporary).ok()?;
    file.write_all(jpeg).ok()?;
    drop(file);
    std::fs::rename(&temporary, &path).ok()?;
    Some(path)
}

pub fn remove_previews(key: &str) {
    for stream in [0u32, STREAM_CONTROLLER, STREAM_HOST, 9] {
        let path = preview_path(key, stream);
        let _ = std::fs::remove_file(path.with_extension("jpg.part"));
        let _ = std::fs::remove_file(path);
    }
}

pub type Outgoing = Arc<dyn Fn(Message) + Send + Sync>;

struct Runtime {
    machine: Machine,
    signal_tx: Option<Outgoing>,
    video_tx: Option<Outgoing>,
    cm: Option<hbb_common::tokio::sync::mpsc::UnboundedSender<crate::ipc::Data>>,
    ui: Option<hbb_common::tokio::sync::mpsc::UnboundedSender<crate::client::Data>>,
    stream_id: u32,
    preview_key: String,
    // Per-call bounded decode admission; no process-global starvation.
    decode_busy: Arc<AtomicBool>,
    last_remote_seq: Option<u32>,
    camera_input: Option<String>,
}

struct CaptureSlot {
    call_id: String,
    stop: Arc<AtomicBool>,
}

lazy_static::lazy_static! {
    static ref HOSTS: Mutex<HashMap<i32, Runtime>> = Mutex::new(HashMap::new());
    static ref CONTROLLERS: Mutex<HashMap<String, Runtime>> = Mutex::new(HashMap::new());
    static ref CAPTURE: Mutex<Option<CaptureSlot>> = Mutex::new(None);
    static ref CAMERA_DEVICE: Mutex<()> = Mutex::new(());
    static ref SEQ: AtomicU32 = AtomicU32::new(1);
}

fn snapshot_phase(machine: &Machine) -> (String, bool, bool, bool) {
    (
        machine.phase.as_str().to_owned(),
        machine.local_camera,
        machine.remote_camera,
        machine.phase.in_call(),
    )
}

fn apply_effect(rt: &mut Runtime, effect: RemoteEffect) {
    if let RemoteEffect::Reply(sig) = effect {
        send_signal(rt, &sig);
    }
}

fn send_signal(rt: &Runtime, sig: &Signal) {
    if let Some(tx) = &rt.signal_tx {
        tx(signal_to_message(sig));
    }
}

fn publish_cm(rt: &Runtime) {
    let Some(cm) = &rt.cm else {
        return;
    };
    let _ = cm.send(crate::ipc::Data::KomiVideoCallState {
        phase: rt.machine.phase.as_str().to_owned(),
        reason: String::new(),
        local_camera: rt.machine.local_camera,
        remote_camera: rt.machine.remote_camera,
        call_id: rt.machine.call_id.clone(),
    });
}

fn publish_ui(rt: &Runtime) {
    let Some(ui) = &rt.ui else {
        return;
    };
    let _ = ui.send(crate::client::Data::KomiVideoCallState(
        rt.machine.phase.as_str().to_owned(),
        rt.machine.local_camera,
        rt.machine.remote_camera,
        rt.machine.call_id.clone(),
    ));
}

fn stop_local_camera(call_id: &str) {
    let mut slot = CAPTURE.lock().unwrap();
    if let Some(cur) = slot.as_ref() {
        if !call_id.is_empty() && cur.call_id == call_id {
            cur.stop.store(true, Ordering::SeqCst);
            *slot = None;
        }
    }
}

// All worker results are revalidated under the owning connection lock. Holding
// this lock through publication serializes the last image with hangup/cleanup.
fn with_live_runtime(key: &str, stream_id: u32, call_id: &str, f: impl FnOnce(&mut Runtime)) {
    if stream_id == STREAM_HOST {
        let mut map = HOSTS.lock().unwrap();
        if let Some(rt) = map.values_mut().find(|rt| rt.preview_key == key) {
            if rt.machine.call_id == call_id
                && matches!(rt.machine.phase, Phase::Outgoing | Phase::Active)
            {
                f(rt);
            }
        }
    } else {
        let mut map = CONTROLLERS.lock().unwrap();
        if let Some(rt) = map.values_mut().find(|rt| rt.preview_key == key) {
            if rt.machine.call_id == call_id
                && matches!(rt.machine.phase, Phase::Outgoing | Phase::Active)
            {
                f(rt);
            }
        }
    }
}

type Preview = Arc<dyn Fn(&[u8], u32, u32, u32) + Send + Sync>;

fn ensure_capture(rt: &Runtime) {
    if !rt.machine.local_camera || !matches!(rt.machine.phase, Phase::Outgoing | Phase::Active) {
        stop_local_camera(rt.machine.call_id());
        return;
    }
    let Some(video_tx) = rt.video_tx.clone() else {
        log::warn!("Komi video call has no sender for camera frames");
        return;
    };
    let call_id = rt.machine.call_id.clone();
    let stream_id = rt.stream_id;
    {
        let slot = CAPTURE.lock().unwrap();
        if let Some(cur) = slot.as_ref() {
            if cur.call_id == call_id && !cur.stop.load(Ordering::SeqCst) {
                return;
            }
            if cur.call_id != call_id && !cur.stop.load(Ordering::SeqCst) {
                log::info!("Komi video call camera is already in use");
                return;
            }
        }
    }
    let stop = Arc::new(AtomicBool::new(false));
    let send_stop = stop.clone();
    let send_key = rt.preview_key.clone();
    let send_call = call_id.clone();
    let gated_tx: Outgoing = Arc::new(move |msg| {
        with_live_runtime(&send_key, stream_id, &send_call, |rt| {
            // The final gate is here, not at capture startup: outgoing preview
            // must never enqueue a network frame before ACCEPT.
            if !send_stop.load(Ordering::SeqCst)
                && rt.machine.local_camera
                && rt.machine.phase == Phase::Active
            {
                video_tx(msg);
            }
        });
    });
    let preview_key = rt.preview_key.clone();
    let preview_call = call_id.clone();
    let preview_stop = stop.clone();
    let preview: Preview = Arc::new(move |jpeg, seq, w, h| {
        with_live_runtime(&preview_key, stream_id, &preview_call, |rt| {
            if preview_stop.load(Ordering::SeqCst) || !rt.machine.local_camera {
                return;
            }
            if let Some(cm) = &rt.cm {
                let _ = cm.send(crate::ipc::Data::KomiCallCameraJpeg {
                    conn_id: 0,
                    call_id: preview_call.clone(),
                    stream_id,
                    seq,
                    width: w,
                    height: h,
                    jpeg: bytes::Bytes::copy_from_slice(jpeg),
                });
            }
            if let Some(ui) = &rt.ui {
                // Stream 0 is the local preview on the controller.
                if let Some(path) = write_preview(&preview_key, 0, jpeg) {
                    let _ = ui.send(crate::client::Data::KomiVideoPreview(
                        path.to_string_lossy().to_string(),
                        seq,
                        0,
                        w,
                        h,
                    ));
                }
            }
        });
    });
    match spawn_capture(call_id.clone(), stream_id, gated_tx, preview, stop.clone(), rt.camera_input.clone()) {
        Ok(()) => {
            *CAPTURE.lock().unwrap() = Some(CaptureSlot { call_id, stop });
        }
        Err(err) => log::warn!("Komi video call capture did not start: {err}"),
    }
}

fn after_local_change(rt: &mut Runtime) {
    if matches!(rt.machine.phase, Phase::Idle | Phase::Ended) {
        rt.last_remote_seq = None;
        remove_previews(&rt.preview_key);
    }
    if !rt.machine.local_camera {
        stop_local_camera(rt.machine.call_id());
    } else {
        ensure_capture(rt);
    }
    publish_cm(rt);
    publish_ui(rt);
}

pub fn host_bind(
    conn_id: i32,
    signal_tx: Outgoing,
    video_tx: Outgoing,
    cm: hbb_common::tokio::sync::mpsc::UnboundedSender<crate::ipc::Data>,
) {
    let mut hosts = HOSTS.lock().unwrap();
    let rt = hosts.entry(conn_id).or_insert_with(|| Runtime {
        machine: Machine::new(),
        signal_tx: None,
        video_tx: None,
        cm: None,
        ui: None,
        stream_id: STREAM_HOST,
        preview_key: format!("host-{conn_id}"),
        decode_busy: Arc::new(AtomicBool::new(false)),
        last_remote_seq: None,
        camera_input: None,
    });
    rt.signal_tx = Some(signal_tx);
    rt.video_tx = Some(video_tx);
    rt.cm = Some(cm);
    rt.stream_id = STREAM_HOST;
}

fn cut_other(rt: &mut Runtime, show: bool) {
    let other = rt.machine.is_show_camera() != show
        && !matches!(rt.machine.phase(), Phase::Idle | Phase::Ended);
    if other {
        let sig = rt.machine.hangup();
        send_signal(rt, &sig);
    }
}

pub fn host_active_video(conn_id: i32) -> bool {
    live_video(HOSTS.lock().unwrap().get(&conn_id), true)
}

pub fn controller_active_video(session: &str) -> bool {
    live_video(CONTROLLERS.lock().unwrap().get(session), true)
}

pub fn host_video_busy(conn_id: i32) -> bool {
    live_video(HOSTS.lock().unwrap().get(&conn_id), false)
}

pub fn controller_video_busy(session: &str) -> bool {
    live_video(CONTROLLERS.lock().unwrap().get(session), false)
}

fn live_video(rt: Option<&Runtime>, active_only: bool) -> bool {
    let Some(rt) = rt else {
        return false;
    };
    if rt.machine.is_show_camera() {
        return false;
    }
    if active_only {
        rt.machine.phase == Phase::Active
    } else {
        matches!(
            rt.machine.phase,
            Phase::Outgoing | Phase::Incoming | Phase::Active
        )
    }
}

pub fn host_command(conn_id: i32, cmd: &str, enabled: bool) {
    // Discovery may block; perform it outside runtime/capture locks. This is
    // local IPC, never peer signaling and never an error-reason payload.
    if let Some(payload) = cmd.strip_prefix("video-input:") {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else { return; };
        let (Some(input), Some(call_id)) = (value["input"].as_str(), value["call_id"].as_str()) else { return; };
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        if scrap::camera::Cameras::input_devices()
            .map(|devices| devices.iter().any(|(id, _)| id == input))
            .unwrap_or(false)
        {
            let mut hosts = HOSTS.lock().unwrap();
            if let Some(rt) = hosts.get_mut(&conn_id) {
                if rt.machine.phase == Phase::Active
                    && rt.machine.call_id() == call_id && !call_id.is_empty() {
                    // Invalidate old publication before starting a replacement.
                    // CAMERA_DEVICE waits until the old worker drops its device.
                    // Selecting an input never grants camera-on consent.
                    rt.camera_input = Some(input.to_owned());
                    if rt.machine.local_camera {
                        stop_local_camera(rt.machine.call_id());
                        ensure_capture(rt);
                    }
                }
            }
        }
        return;
    }
    let mut hosts = HOSTS.lock().unwrap();
    let Some(rt) = hosts.get_mut(&conn_id) else {
        return;
    };
    let now = now_ms();
    let result = match cmd {
        "request" => {
            cut_other(rt, false);
            rt.machine.request(now).map(|sig| {
                send_signal(rt, &sig);
            })
        }
        "show-mine" => {
            if rt.machine.is_show_camera()
                && !matches!(rt.machine.phase(), Phase::Idle | Phase::Ended)
            {
                let sig = rt.machine.hangup();
                send_signal(rt, &sig);
                Ok(())
            } else {
                cut_other(rt, true);
                rt.machine.request_show(now).map(|sig| {
                    send_signal(rt, &sig);
                })
            }
        }
        "accept" => rt.machine.accept(now).map(|sig| {
            send_signal(rt, &sig);
        }),
        "refuse" => rt.machine.refuse().map(|sig| {
            send_signal(rt, &sig);
        }),
        "hangup" => {
            let sig = rt.machine.hangup();
            send_signal(rt, &sig);
            Ok(())
        }
        "camera" => if enabled {
            rt.machine.camera_on()
        } else {
            rt.machine.camera_off()
        }
        .map(|sig| send_signal(rt, &sig)),
        _ => Err("bad-cmd"),
    };
    if let Err(err) = result {
        log::info!("Komi video call host command {cmd} ignored: {err}");
    }
    after_local_change(rt);
}

pub fn host_on_call(conn_id: i32, call: &KomiVideoCall) {
    let mut hosts = HOSTS.lock().unwrap();
    let Some(rt) = hosts.get_mut(&conn_id) else {
        return;
    };
    let effect = rt.machine.on_remote(&message_to_signal(call), now_ms());
    apply_effect(rt, effect);
    after_local_change(rt);
}

pub fn host_on_frame(conn_id: i32, mut frame: KomiCallCameraFrame) {
    let (allow, cm, key, busy) = {
        let mut hosts = HOSTS.lock().unwrap();
        let Some(rt) = hosts.get_mut(&conn_id) else {
            return;
        };
        let vf = match frame.frame.as_ref() {
            Some(vf) => vf,
            None => return,
        };
        let n = video_frame_len(vf);
        let was_remote = rt.machine.remote_camera;
        let allow =
            frame.stream_id == STREAM_CONTROLLER && rt.machine.allow_frame(&frame.call_id, n);
        if allow && !was_remote {
            publish_cm(rt);
        }
        (
            allow,
            rt.cm.clone(),
            rt.preview_key.clone(),
            rt.decode_busy.clone(),
        )
    };
    if !allow {
        return;
    }
    let Some(vf) = frame.frame.take() else {
        return;
    };
    let Some(cm) = cm else {
        return;
    };
    decode_and_forward(
        conn_id,
        key,
        frame.call_id,
        frame.stream_id,
        frame.seq,
        vf,
        cm,
        busy,
    );
}

pub fn host_tick(conn_id: i32) {
    let mut hosts = HOSTS.lock().unwrap();
    let Some(rt) = hosts.get_mut(&conn_id) else {
        return;
    };
    if let Some(sig) = rt.machine.tick(now_ms()) {
        send_signal(rt, &sig);
        after_local_change(rt);
    }
}

pub fn host_stop(conn_id: i32) {
    let mut hosts = HOSTS.lock().unwrap();
    if let Some(mut rt) = hosts.remove(&conn_id) {
        if rt.machine.phase != Phase::Idle && rt.machine.phase != Phase::Ended {
            let sig = rt.machine.hangup();
            send_signal(&rt, &sig);
        }
        stop_local_camera(&rt.machine.call_id);
        remove_previews(&rt.preview_key);
        rt.machine = Machine::new();
        publish_cm(&rt);
    }
}

pub fn host_in_video_call(conn_id: i32) -> bool {
    HOSTS
        .lock()
        .unwrap()
        .get(&conn_id)
        .map(|rt| rt.machine.phase.in_call())
        .unwrap_or(false)
}

pub fn host_frame_live(conn_id: i32, frame: &KomiCallCameraFrame) -> bool {
    HOSTS
        .lock()
        .unwrap()
        .get(&conn_id)
        .map(|rt| {
            rt.machine.phase == Phase::Active
                && rt.machine.local_camera
                && rt.machine.call_id == frame.call_id
                && frame.stream_id == STREAM_HOST
        })
        .unwrap_or(false)
}

pub fn controller_frame_live(session: &str, frame: &KomiCallCameraFrame) -> bool {
    CONTROLLERS
        .lock()
        .unwrap()
        .get(session)
        .map(|rt| {
            rt.machine.phase == Phase::Active
                && rt.machine.local_camera
                && rt.machine.call_id == frame.call_id
                && frame.stream_id == STREAM_CONTROLLER
        })
        .unwrap_or(false)
}

pub fn controller_bind(
    session: &str,
    signal_and_ui: hbb_common::tokio::sync::mpsc::UnboundedSender<crate::client::Data>,
) {
    let ui = signal_and_ui.clone();
    let signal_tx: Outgoing = Arc::new(move |msg: Message| {
        let _ = ui.send(crate::client::Data::Message(msg));
    });
    let video_tx = signal_tx.clone();
    let mut map = CONTROLLERS.lock().unwrap();
    let rt = map.entry(session.to_owned()).or_insert_with(|| Runtime {
        machine: Machine::new(),
        signal_tx: None,
        video_tx: None,
        cm: None,
        ui: None,
        stream_id: STREAM_CONTROLLER,
        preview_key: format!("ctrl-{session}"),
        decode_busy: Arc::new(AtomicBool::new(false)),
        last_remote_seq: None,
        camera_input: None,
    });
    rt.signal_tx = Some(signal_tx);
    rt.video_tx = Some(video_tx);
    rt.ui = Some(signal_and_ui);
    rt.stream_id = STREAM_CONTROLLER;
}

pub fn controller_command(session: &str, cmd: &str) {
    let mut map = CONTROLLERS.lock().unwrap();
    let Some(rt) = map.get_mut(session) else {
        return;
    };
    let now = now_ms();
    let result = match cmd {
        "request" => {
            cut_other(rt, false);
            rt.machine.request(now).map(|sig| send_signal(rt, &sig))
        }
        "show-mine" => {
            if rt.machine.is_show_camera()
                && !matches!(rt.machine.phase(), Phase::Idle | Phase::Ended)
            {
                let sig = rt.machine.hangup();
                send_signal(rt, &sig);
                Ok(())
            } else {
                cut_other(rt, true);
                rt.machine.request_show(now).map(|sig| send_signal(rt, &sig))
            }
        }
        "accept" => rt.machine.accept(now).map(|sig| send_signal(rt, &sig)),
        "refuse" => rt.machine.refuse().map(|sig| send_signal(rt, &sig)),
        "hangup" => {
            let sig = rt.machine.hangup();
            send_signal(rt, &sig);
            Ok(())
        }
        "camera-on" => rt.machine.camera_on().map(|sig| send_signal(rt, &sig)),
        "camera-off" => rt.machine.camera_off().map(|sig| send_signal(rt, &sig)),
        _ => Err("bad-cmd"),
    };
    if let Err(err) = result {
        log::info!("Komi video call controller command {cmd} ignored: {err}");
    }
    after_local_change(rt);
}

pub fn controller_on_call(session: &str, call: &KomiVideoCall) {
    let mut map = CONTROLLERS.lock().unwrap();
    let Some(rt) = map.get_mut(session) else {
        return;
    };
    let effect = rt.machine.on_remote(&message_to_signal(call), now_ms());
    apply_effect(rt, effect);
    after_local_change(rt);
}

pub fn controller_on_frame(session: &str, mut frame: KomiCallCameraFrame) {
    let (allow, ui, key, busy) = {
        let mut map = CONTROLLERS.lock().unwrap();
        let Some(rt) = map.get_mut(session) else {
            return;
        };
        let vf = match frame.frame.as_ref() {
            Some(vf) => vf,
            None => return,
        };
        let was_remote = rt.machine.remote_camera;
        let allow = frame.stream_id == STREAM_HOST
            && rt.machine.allow_frame(&frame.call_id, video_frame_len(vf));
        if allow && !was_remote {
            publish_ui(rt);
        }
        (
            allow,
            rt.ui.clone(),
            rt.preview_key.clone(),
            rt.decode_busy.clone(),
        )
    };
    if !allow {
        return;
    }
    let Some(vf) = frame.frame.take() else {
        return;
    };
    let Some(ui) = ui else {
        return;
    };
    decode_and_preview(key, frame.call_id, frame.stream_id, frame.seq, vf, ui, busy);
}

pub fn controller_tick(session: &str) {
    let mut map = CONTROLLERS.lock().unwrap();
    let Some(rt) = map.get_mut(session) else {
        return;
    };
    if let Some(sig) = rt.machine.tick(now_ms()) {
        send_signal(rt, &sig);
        after_local_change(rt);
    }
}

pub fn controller_stop(session: &str) {
    let mut map = CONTROLLERS.lock().unwrap();
    if let Some(mut rt) = map.remove(session) {
        if rt.machine.phase != Phase::Idle && rt.machine.phase != Phase::Ended {
            let sig = rt.machine.hangup();
            send_signal(&rt, &sig);
        }
        stop_local_camera(&rt.machine.call_id);
        remove_previews(&rt.preview_key);
    }
}

struct DecodePermit(Arc<AtomicBool>);
impl Drop for DecodePermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
fn admit_decode(busy: Arc<AtomicBool>) -> Option<DecodePermit> {
    busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()?;
    Some(DecodePermit(busy))
}
fn fresh_remote(rt: &mut Runtime, seq: u32) -> bool {
    if rt.machine.phase != Phase::Active {
        return false;
    }
    if let Some(last) = rt.last_remote_seq {
        let distance = seq.wrapping_sub(last);
        if distance == 0 || distance >= (1u32 << 31) {
            return false;
        }
    }
    rt.last_remote_seq = Some(seq);
    true
}

fn decode_and_forward(
    conn_id: i32,
    key: String,
    call_id: String,
    stream_id: u32,
    seq: u32,
    frame: VideoFrame,
    cm: hbb_common::tokio::sync::mpsc::UnboundedSender<crate::ipc::Data>,
    busy: Arc<AtomicBool>,
) {
    let Some(permit) = admit_decode(busy) else {
        return;
    };
    std::thread::spawn(move || {
        let _permit = permit;
        let Some((jpeg, w, h)) = decode_jpeg(&frame) else {
            return;
        };
        with_live_runtime(&key, STREAM_HOST, &call_id, |rt| {
            if !fresh_remote(rt, seq) {
                return;
            }
            let _ = cm.send(crate::ipc::Data::KomiCallCameraJpeg {
                conn_id,
                call_id: call_id.clone(),
                stream_id,
                seq,
                width: w,
                height: h,
                jpeg: bytes::Bytes::from(jpeg),
            });
        });
    });
}

fn decode_and_preview(
    key: String,
    call_id: String,
    stream_id: u32,
    seq: u32,
    frame: VideoFrame,
    ui: hbb_common::tokio::sync::mpsc::UnboundedSender<crate::client::Data>,
    busy: Arc<AtomicBool>,
) {
    let Some(permit) = admit_decode(busy) else {
        return;
    };
    std::thread::spawn(move || {
        let _permit = permit;
        let Some((jpeg, w, h)) = decode_jpeg(&frame) else {
            return;
        };
        with_live_runtime(&key, STREAM_CONTROLLER, &call_id, |rt| {
            if !fresh_remote(rt, seq) {
                return;
            }
            if let Some(path) = write_preview(&key, stream_id, &jpeg) {
                let _ = ui.send(crate::client::Data::KomiVideoPreview(
                    path.to_string_lossy().to_string(),
                    seq,
                    stream_id,
                    w,
                    h,
                ));
            }
        });
    });
}

fn decode_jpeg(frame: &VideoFrame) -> Option<(Vec<u8>, u32, u32)> {
    let union = frame.union.as_ref()?;
    if !matches!(union, video_frame::Union::Vp8s(_)) {
        log::debug!("Komi video call ignored a non-VP8 camera frame");
        return None;
    }
    let mut decoder = scrap::codec::Decoder::new(scrap::CodecFormat::VP8, None);
    if !decoder.valid() {
        return None;
    }
    let fmt = if cfg!(feature = "flutter") && (cfg!(windows) || cfg!(target_os = "linux")) {
        scrap::ImageFormat::ABGR
    } else {
        scrap::ImageFormat::ARGB
    };
    let mut rgb = scrap::ImageRgb::new(fmt, crate::get_dst_align_rgba());
    let mut texture = scrap::ImageTexture::default();
    let mut pixel = true;
    let mut chroma = None;
    match decoder.handle_video_frame(union, &mut rgb, &mut texture, &mut pixel, &mut chroma) {
        Ok(true) => {
            let (w, h) = (rgb.w as u32, rgb.h as u32);
            decoded_to_jpeg(&rgb).map(|jpeg| (jpeg, w, h))
        }
        _ => None,
    }
}

fn spawn_capture(
    call_id: String,
    stream_id: u32,
    video_tx: Outgoing,
    preview: Preview,
    stop: Arc<AtomicBool>,
    camera_input: Option<String>,
) -> Result<(), String> {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        let stop_thread = stop.clone();
        std::thread::Builder::new()
            .name("komi-video-call".to_owned())
            .spawn(move || capture_loop(call_id, stream_id, video_tx, preview, stop_thread, camera_input))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = (call_id, stream_id, video_tx, preview, stop, camera_input);
        Err("camera unsupported on this platform".to_owned())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn capture_loop(
    call_id: String,
    stream_id: u32,
    video_tx: Outgoing,
    preview: Preview,
    stop: Arc<AtomicBool>,
    camera_input: Option<String>,
) {
    // A stopped worker can still be inside a blocking camera read. Never open
    // the replacement device until that worker has actually dropped it.
    let _device = CAMERA_DEVICE.lock().unwrap();
    struct Exit(Arc<AtomicBool>);
    impl Drop for Exit {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let _exit = Exit(stop.clone());
    if stop.load(Ordering::SeqCst) {
        return;
    }
    let device = match camera_input.as_deref() {
        Some(id) => scrap::camera::Cameras::get_input_capturer(id),
        None => scrap::camera::Cameras::get_capturer(0),
    };
    let mut capturer = match device {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Komi video call camera open failed: {e}");
            return;
        }
    };
    let mut encoder: Option<scrap::codec::Encoder> = None;
    let mut enc_size = (0u32, 0u32);
    let mut yuv = Vec::new();
    let mut mid = Vec::new();
    let mut seq = 0u32;
    let interval = Duration::from_millis(1000 / MAX_FPS as u64);
    let mut next = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now < next {
            std::thread::sleep(next - now);
            continue;
        }
        next = Instant::now() + interval;
        let frame = match capturer.frame(Duration::from_millis(80)) {
            Ok(frame) => frame,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(err) => {
                log::warn!("Komi video call capture stopped: {err}");
                break;
            }
        };
        let scrap::Frame::PixelBuffer(buf) = &frame else {
            continue;
        };
        let (dw, dh) = fit_even(
            buf.width() as u32,
            buf.height() as u32,
            MAX_WIDTH,
            MAX_HEIGHT,
        );
        let scaled = if buf.width() == dw as usize && buf.height() == dh as usize {
            buf.data().to_vec()
        } else {
            scale_rgba(
                buf.data(),
                buf.width(),
                buf.height(),
                dw as usize,
                dh as usize,
            )
        };
        if stop.load(Ordering::SeqCst) {
            break;
        }
        seq = SEQ.fetch_add(1, Ordering::Relaxed);
        if let Some(jpeg) = rgba_to_jpeg(&scaled, dw, dh) {
            preview(&jpeg, seq, dw, dh);
        }
        if enc_size != (dw, dh) || encoder.is_none() {
            match scrap::codec::Encoder::new(
                scrap::codec::EncoderCfg::VPX(scrap::vpxcodec::VpxEncoderConfig {
                    width: dw,
                    height: dh,
                    quality: 0.2,
                    codec: scrap::vpxcodec::VpxVideoCodecId::VP8,
                    keyframe_interval: Some(1),
                }),
                false,
            ) {
                Ok(enc) => {
                    encoder = Some(enc);
                    enc_size = (dw, dh);
                }
                Err(err) => {
                    log::warn!("Komi video call encoder failed: {err}");
                    break;
                }
            }
        }
        let Some(enc) = encoder.as_mut() else {
            break;
        };
        let yuvfmt = enc.yuvfmt();
        let pb = scrap::PixelBuffer::new(&scaled, scrap::Pixfmt::RGBA, dw as usize, dh as usize);
        let owned = scrap::Frame::PixelBuffer(pb);
        let input = match owned.to(yuvfmt, &mut yuv, &mut mid) {
            Ok(input) => input,
            Err(err) => {
                log::debug!("Komi video call convert failed: {err}");
                continue;
            }
        };
        let vf = match enc.encode_to_message(input, now_ms()) {
            Ok(vf) => vf,
            Err(err) => {
                log::debug!("Komi video call encode failed: {err}");
                continue;
            }
        };
        let n = video_frame_len(&vf);
        // The receiver intentionally admits/drops independent frames and creates
        // a decoder per image. Never send an interframe to that pipeline even
        // if a codec configuration/driver fails to honor kf_max_dist=1.
        if !frame_is_key(&vf) {
            log::debug!("Komi video call restarting encoder after non-key output");
            encoder = None;
            continue;
        }
        if n == 0 || n > MAX_FRAME_BYTES {
            log::debug!("Komi video call dropped a {n} byte frame");
            continue;
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        video_tx(frame_message(&call_id, stream_id, seq, vf));
    }
    drop(capturer);
    log::info!("Komi video call capture stopped");
}

/// Used by the CM process after the JPEG has crossed IPC as raw bytes.
pub fn cm_store_jpeg(conn_id: i32, stream_id: u32, jpeg: &[u8]) -> Option<PathBuf> {
    write_preview(&format!("cm-{conn_id}"), stream_id, jpeg)
}

pub fn cm_clear(conn_id: i32) {
    remove_previews(&format!("cm-{conn_id}"));
}

#[allow(dead_code)]
fn _keep_encoded_types(frame: EncodedVideoFrame, frames: EncodedVideoFrames) -> usize {
    frame.data.len() + frames.frames.len()
}

#[allow(dead_code)]
fn _seq() -> u32 {
    SEQ.fetch_add(1, Ordering::Relaxed)
}

#[allow(dead_code)]
fn _phase_tuple(machine: &Machine) -> (String, bool, bool, bool) {
    snapshot_phase(machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_on_initiative_transmission_after_accept() {
        let mut m = Machine::new();
        assert!(m.camera_on().is_err());
        let req = m.request(1_000).unwrap();
        assert!(m.local_camera());
        assert!(m.camera_on().is_err());
        let mut peer = Machine::new();
        assert!(matches!(peer.on_remote(&req, 1_100), RemoteEffect::Changed));
        assert_eq!(peer.phase(), Phase::Incoming);
        assert!(!peer.local_camera());
        assert!(peer.camera_on().is_err());
        let accept = peer.accept(1_200).unwrap();
        assert!(peer.local_camera());
        assert!(matches!(m.on_remote(&accept, 1_300), RemoteEffect::Changed));
        assert_eq!(m.phase(), Phase::Active);
        assert!(m.camera_on().is_ok());
        assert!(m.local_camera());
    }

    #[test]
    fn refuse_and_expire_do_not_open_a_camera() {
        let mut host = Machine::new();
        let mut ctrl = Machine::new();
        let req = ctrl.request(0).unwrap();
        host.on_remote(&req, 10);
        let refused = host.refuse().unwrap();
        ctrl.on_remote(&refused, 20);
        assert_eq!(ctrl.phase(), Phase::Ended);
        assert!(ctrl.camera_on().is_err());

        let mut waiting = Machine::new();
        let req = waiting.request(0).unwrap();
        assert!(waiting.tick(req.expires_ms).is_none());
        let expired = waiting.tick(req.expires_ms + 1).unwrap();
        assert_eq!(expired.action, ACTION_EXPIRE);
        assert!(!waiting.local_camera());
    }

    #[test]
    fn mismatched_accept_is_ignored() {
        let mut m = Machine::new();
        let req = m.request(0).unwrap();
        let mut forged = req.clone();
        forged.action = ACTION_ACCEPT;
        forged.nonce = req.nonce.wrapping_add(1);
        assert!(matches!(m.on_remote(&forged, 10), RemoteEffect::None));
        assert_eq!(m.phase(), Phase::Outgoing);
        forged.nonce = req.nonce;
        forged.call_id = "other".to_owned();
        assert!(matches!(m.on_remote(&forged, 10), RemoteEffect::None));
        assert_eq!(m.phase(), Phase::Outgoing);
    }

    #[test]
    fn frames_need_the_accepted_call_and_a_size_cap() {
        let mut m = Machine::new();
        assert!(!m.allow_frame("x", 10));
        let req = m.request(0).unwrap();
        let mut peer = Machine::new();
        peer.on_remote(&req, 1);
        let accept = peer.accept(2).unwrap();
        m.on_remote(&accept, 3);
        assert!(!m.allow_frame("nope", 10));
        assert!(!m.allow_frame(&req.call_id, 0));
        assert!(!m.allow_frame(&req.call_id, MAX_FRAME_BYTES + 1));
        assert!(m.allow_frame(&req.call_id, 32));
        m.hangup();
        assert!(!m.allow_frame(&req.call_id, 32));
    }

    #[test]
    fn busy_while_a_call_is_open() {
        let mut m = Machine::new();
        m.request(0).unwrap();
        assert_eq!(m.request(1).unwrap_err(), "busy");
        let foreign = Signal {
            call_id: "other".into(),
            version: VERSION,
            action: ACTION_REQUEST,
            reason: String::new(),
            nonce: 7,
            expires_ms: 50_000,
        };
        match m.on_remote(&foreign, 10) {
            RemoteEffect::Reply(sig) => assert_eq!(sig.action, ACTION_BUSY),
            other => panic!("expected busy, got {other:?}"),
        }
    }

    #[test]
    fn unknown_version_is_refused() {
        let mut m = Machine::new();
        let sig = Signal {
            call_id: "abc".into(),
            version: 99,
            action: ACTION_REQUEST,
            reason: String::new(),
            nonce: 3,
            expires_ms: 10_000,
        };
        match m.on_remote(&sig, 1) {
            RemoteEffect::Reply(sig) => assert_eq!(sig.reason, "unsupported"),
            other => panic!("expected unsupported, got {other:?}"),
        }
        assert_eq!(m.phase(), Phase::Idle);
    }

    #[test]
    fn fit_even_caps_and_stays_even() {
        assert_eq!(fit_even(1920, 1080, 640, 360), (640, 360));
        assert_eq!(fit_even(100, 100, 640, 360), (100, 100));
        let (w, h) = fit_even(641, 361, 640, 360);
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
        assert!(w <= 640 && h <= 360);
    }

    #[test]
    fn idle_connection_does_not_stop_another_capture() {
        let stop = Arc::new(AtomicBool::new(false));
        *CAPTURE.lock().unwrap() = Some(CaptureSlot {
            call_id: "owned-test".into(),
            stop: stop.clone(),
        });
        stop_local_camera("");
        assert!(!stop.load(Ordering::SeqCst));
        stop_local_camera("other");
        assert!(!stop.load(Ordering::SeqCst));
        stop_local_camera("owned-test");
        assert!(stop.load(Ordering::SeqCst));
    }

    #[test]
    fn decode_admission_is_per_connection_and_released() {
        let busy = Arc::new(AtomicBool::new(false));
        let permit = admit_decode(busy.clone()).unwrap();
        assert!(admit_decode(busy.clone()).is_none());
        assert!(admit_decode(Arc::new(AtomicBool::new(false))).is_some());
        drop(permit);
        assert!(admit_decode(busy).is_some());
    }

    #[test]
    fn tiny_rgba_encodes_to_jpeg() {
        let jpeg = rgba_to_jpeg(
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 1, 2, 3, 255],
            2,
            2,
        )
        .expect("jpeg");
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    }
}
