//! KomiRemote: host-side session pills (Windows, CM process).
//!
//! Mini: 32px rounded icon buttons in one horizontal bar, themed like the
//! session toolbar. Hover unfolds one shorter pill per line, with the short
//! labels Contrôle / Appel · mm:ss / Caméra. The hang-up control is a red
//! icon with no text. The window stays topmost, never takes focus, and is
//! excluded from capture so the controller neither sees nor clicks it.

use crate::ui_cm_interface::{self, KomiPillClient};
use hbb_common::log;
use std::{
    collections::HashMap,
    ptr::null_mut,
    sync::{Mutex, Once},
    time::{Duration, Instant},
};
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::{HBITMAP, HDC, HGDIOBJ, HWND, POINT, RECT},
    },
    um::{
        wingdi::{
            CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen,
            CreateSolidBrush, DeleteDC, DeleteObject, GetDeviceCaps, GetDIBits, RoundRect,
            SelectObject, SetBkMode, SetTextColor, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
            BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DIB_RGB_COLORS,
            DEFAULT_CHARSET, DEFAULT_PITCH, FW_SEMIBOLD, LOGPIXELSY, OUT_DEFAULT_PRECIS, PS_SOLID,
            RGB, TRANSPARENT,
        },
        winuser::*,
    },
};

#[link(name = "user32")]
extern "system" {
    fn SetWindowDisplayAffinity(hwnd: HWND, affinity: u32) -> i32;
    fn GetCurrentInputMessageSource(source: *mut InputMessageSource) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut std::ffi::c_void;
    fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
}

#[repr(C)]
struct InputMessageSource {
    device_type: u32,
    origin_id: u32,
}

unsafe fn hardware_mouse() -> bool {
    let mut source = InputMessageSource {
        device_type: 0,
        origin_id: 0,
    };
    GetCurrentInputMessageSource(&mut source) != 0
        && source.device_type == 2
        && source.origin_id == 1
}

const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;
const TIMER_ID: usize = 1;
const COLOR_KEY: u32 = 0x00FF00FF;
const AC_SRC_OVER: u8 = 0;
const AC_SRC_ALPHA: u8 = 1;

#[repr(C)]
struct BlendFunction {
    blend_op: u8,
    blend_flags: u8,
    source_constant_alpha: u8,
    alpha_format: u8,
}

#[link(name = "msimg32")]
extern "system" {
    fn AlphaBlend(
        dst: HDC,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: HDC,
        sx: i32,
        sy: i32,
        sw: i32,
        sh: i32,
        blend: BlendFunction,
    ) -> i32;
}

include!("komi_pill_glyphs.rs");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Control,
    Call,
    VideoCall,
    Camera,
}

#[derive(Clone)]
struct Item {
    id: i32,
    kind: Kind,
    text: String,
}

struct State {
    items: Vec<Item>,
    hover: Option<usize>,
    call_start: HashMap<i32, Instant>,
    dark: bool,
    ticks: u32,
    shown_since: Instant,
    armed: Option<(i32, Kind)>,
    position: Option<(i32, i32)>,
    drag: Option<(i32, i32)>,
}

lazy_static::lazy_static! {
    static ref STATE: Mutex<State> = Mutex::new(State {
        items: vec![],
        hover: None,
        call_start: HashMap::new(),
        dark: true,
        ticks: 0,
        shown_since: Instant::now(),
        armed: None,
        position: None,
        drag: None,
    });
}

static START: Once = Once::new();

pub fn ensure_started() {
    START.call_once(|| {
        std::thread::spawn(|| unsafe { run() });
    });
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn read_dark() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .map(|v| v == 0)
        .unwrap_or(false)
}

fn tr(key: &str) -> String {
    crate::lang::translate(key.to_owned())
}

fn build_items(clients: &[KomiPillClient], st: &mut State) -> Vec<Item> {
    let mut items = vec![];
    let mut calls = vec![];
    for c in clients {
        if c.is_control && !c.is_view_camera {
            items.push(Item {
                id: c.id,
                kind: Kind::Control,
                text: tr("komi-pill-controlled"),
            });
        }
        if c.in_voice_call {
            calls.push(c.id);
            let start = *st.call_start.entry(c.id).or_insert_with(Instant::now);
            let secs = start.elapsed().as_secs();
            items.push(Item {
                id: c.id,
                kind: Kind::Call,
                text: format!("{} · {:02}:{:02}", tr("komi-pill-call"), secs / 60, secs % 60),
            });
        }
        if c.in_video_call && !c.is_camera_share {
            let secs = c.video_accepted_at.map(|start| start.elapsed().as_secs()).unwrap_or(0);
            items.push(Item {
                id: c.id,
                kind: Kind::VideoCall,
                text: format!("{} · {:02}:{:02}", tr("komi-pill-call"), secs / 60, secs % 60),
            });
        } else if c.is_view_camera || (c.in_video_call && c.is_camera_share) {
            items.push(Item {
                id: c.id,
                kind: Kind::Camera,
                text: tr("komi-pill-camera"),
            });
        }
    }
    items.sort_by_key(|item| (kind_ord(item.kind), item.id));
    st.call_start.retain(|id, _| calls.contains(id));
    items
}

fn kind_ord(kind: Kind) -> u8 {
    match kind {
        Kind::Control => 0,
        Kind::Call => 1,
        Kind::VideoCall => 2,
        Kind::Camera => 3,
    }
}

struct Metrics {
    btn: i32,
    radius: i32,
    gap: i32,
    pad: i32,
    pill_w: i32,
    scale: f32,
}

fn metrics(hwnd: HWND) -> Metrics {
    let dpi = unsafe {
        // Resolve dynamically: do not introduce a hard Windows 10 1607 import
        // or change the process/thread DPI-awareness policy.
        let module = GetModuleHandleW(wide("user32.dll").as_ptr());
        let proc = if module.is_null() {
            null_mut()
        } else {
            GetProcAddress(module, b"GetDpiForWindow\0".as_ptr())
        };
        let window_dpi = if proc.is_null() {
            0
        } else {
            let get_dpi: unsafe extern "system" fn(HWND) -> u32 = std::mem::transmute(proc);
            get_dpi(hwnd)
        };
        if window_dpi > 0 {
            window_dpi as i32
        } else {
            let dc = GetDC(hwnd);
            let dpi = if dc.is_null() { 0 } else { GetDeviceCaps(dc, LOGPIXELSY as _) };
            if !dc.is_null() {
                ReleaseDC(hwnd, dc);
            }
            dpi
        }
    };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let s = |v: f32| (v * scale).round() as i32;
    Metrics {
        btn: s(32.0),
        radius: s(8.0),
        gap: s(4.0),
        pad: s(4.0),
        pill_w: s(196.0),
        scale,
    }
}

fn expanded_now(st: &State) -> bool {
    st.hover.is_some() || st.shown_since.elapsed() < Duration::from_secs(4)
}

fn row_h(m: &Metrics) -> i32 {
    m.pad * 2 + m.btn
}

fn window_size(m: &Metrics, n: usize, expanded: bool) -> (i32, i32) {
    if n == 0 {
        return (1, 1);
    }
    let n = n as i32;
    if expanded {
        (m.pill_w, n * row_h(m) + (n - 1) * m.gap)
    } else {
        (
            m.pad * 2 + n * m.btn + (n - 1) * m.gap,
            m.pad * 2 + m.btn,
        )
    }
}

fn item_at(m: &Metrics, n: usize, x: i32, y: i32, expanded: bool) -> Option<usize> {
    if n == 0 || x < 0 || y < 0 {
        return None;
    }
    if expanded {
        let stride = row_h(m) + m.gap;
        if stride <= 0 {
            return None;
        }
        let index = y / stride;
        if index >= 0 && (index as usize) < n && y % stride < row_h(m) && x < m.pill_w {
            Some(index as usize)
        } else {
            None
        }
    } else {
        if y < m.pad || y >= m.pad + m.btn {
            return None;
        }
        let local = x - m.pad;
        let stride = m.btn + m.gap;
        if local < 0 || stride <= 0 {
            return None;
        }
        let index = local / stride;
        if index >= 0 && (index as usize) < n && local % stride < m.btn {
            Some(index as usize)
        } else {
            None
        }
    }
}

fn in_action(m: &Metrics, x: i32, y: i32, row: usize, expanded: bool) -> bool {
    if !expanded {
        return false;
    }
    let top = row as i32 * (row_h(m) + m.gap) + m.pad;
    x >= m.pill_w - m.pad - m.btn && x < m.pill_w - m.pad && y >= top && y < top + m.btn
}

unsafe fn draw_text(dc: HDC, text: &str, rc: &mut RECT, flags: UINT) {
    let w: Vec<u16> = text.encode_utf16().collect();
    DrawTextW(dc, w.as_ptr(), w.len() as i32, rc, flags);
}

unsafe fn font(size: i32) -> winapi::shared::windef::HFONT {
    let face = wide("Segoe UI");
    CreateFontW(
        -size,
        0,
        0,
        0,
        FW_SEMIBOLD as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET as _,
        OUT_DEFAULT_PRECIS as _,
        CLIP_DEFAULT_PRECIS as _,
        CLEARTYPE_QUALITY as _,
        DEFAULT_PITCH as _,
        face.as_ptr(),
    )
}

struct GlyphCache {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    size: i32,
}

unsafe fn glyph_dc(screen: HDC, index: usize, size: i32) -> Option<GlyphCache> {
    let (px, pixels) = glyph_pixels(index, size)?;
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = px as i32;
    info.bmiHeader.biHeight = -(px as i32);
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB as u32;
    let mut bits = null_mut();
    let bmp = CreateDIBSection(
        screen,
        &info,
        DIB_RGB_COLORS,
        &mut bits,
        null_mut(),
        0,
    );
    if bmp.is_null() || bits.is_null() {
        if !bmp.is_null() {
            DeleteObject(bmp as _);
        }
        return None;
    }
    std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
    let dc = CreateCompatibleDC(screen);
    if dc.is_null() {
        DeleteObject(bmp as _);
        return None;
    }
    let old = SelectObject(dc, bmp as _);
    Some(GlyphCache { dc, bmp, old, size: px as i32 })
}

unsafe fn blit_glyph(dst: HDC, src: &GlyphCache, x: i32, y: i32, size: i32) {
    let blend = BlendFunction {
        blend_op: AC_SRC_OVER,
        blend_flags: 0,
        source_constant_alpha: 255,
        alpha_format: AC_SRC_ALPHA,
    };
    AlphaBlend(
        dst,
        x + (size - src.size) / 2,
        y + (size - src.size) / 2,
        src.size,
        src.size,
        src.dc,
        0,
        0,
        src.size,
        src.size,
        blend,
    );
}

unsafe fn round_fill(dc: HDC, l: i32, t: i32, r: i32, b: i32, radius: i32, fill: u32, edge: u32) {
    let brush = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID as _, 1, edge);
    let ob = SelectObject(dc, brush as _);
    let op = SelectObject(dc, pen as _);
    let d = radius * 2;
    RoundRect(dc, l, t, r, b, d, d);
    SelectObject(dc, ob);
    SelectObject(dc, op);
    DeleteObject(brush as _);
    DeleteObject(pen as _);
}

unsafe fn refresh(hwnd: HWND) {
    let clients = ui_cm_interface::komi_pill_clients();
    let (n, expanded) = {
        let mut st = STATE.lock().unwrap();
        if st.ticks % 8 == 0 {
            st.dark = read_dark();
        }
        st.ticks = st.ticks.wrapping_add(1);
        let items = build_items(&clients, &mut st);
        let changed = items.len() != st.items.len()
            || items
                .iter()
                .zip(&st.items)
                .any(|(a, b)| a.id != b.id || a.kind != b.kind);
        if changed {
            st.shown_since = Instant::now();
            st.hover = None;
            st.armed = None;
        }
        st.items = items;
        let mut pt: POINT = std::mem::zeroed();
        let mut rc: RECT = std::mem::zeroed();
        GetCursorPos(&mut pt);
        GetWindowRect(hwnd, &mut rc);
        if pt.x < rc.left || pt.x >= rc.right || pt.y < rc.top || pt.y >= rc.bottom {
            st.hover = None;
        }
        if let Some(h) = st.hover {
            if h >= st.items.len() {
                st.hover = None;
            }
        }
        (st.items.len(), expanded_now(&st))
    };
    if n == 0 {
        ShowWindow(hwnd, SW_HIDE);
        return;
    }
    let m = metrics(hwnd);
    let (width, height) = window_size(&m, n, expanded);
    let mut work: RECT = std::mem::zeroed();
    if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as _, 0) == 0 {
        work.right = GetSystemMetrics(SM_CXSCREEN);
        work.bottom = GetSystemMetrics(SM_CYSCREEN);
    }
    let margin = (12.0 * m.scale) as i32;
    let position = STATE.lock().unwrap().position;
    let (x, y) = position.unwrap_or((
        work.left + (work.right - work.left - width) / 2,
        work.top + margin,
    ));
    SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE);
    if IsWindowVisible(hwnd) == 0 {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    InvalidateRect(hwnd, null_mut(), 0);
}

unsafe fn present_smooth(hwnd: HWND, hdc: HDC, src_bmp: HBITMAP, cw: i32, ch: i32, scale: i32) {
    if src_bmp.is_null() || cw <= 0 || ch <= 0 || scale <= 0 {
        return;
    }
    let bw = cw * scale;
    let bh = ch * scale;
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = bw;
    info.bmiHeader.biHeight = -bh;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB as u32;
    let mut raw = vec![0u8; (bw as usize) * (bh as usize) * 4];
    if GetDIBits(
        hdc,
        src_bmp,
        0,
        bh as u32,
        raw.as_mut_ptr() as _,
        &mut info,
        DIB_RGB_COLORS,
    ) == 0
    {
        return;
    }
    let mut out = vec![0u8; (cw as usize) * (ch as usize) * 4];
    let s = scale as usize;
    let bw_us = bw as usize;
    let samples = (s * s) as u32;
    for y in 0..ch as usize {
        for x in 0..cw as usize {
            let mut cover = 0u32;
            let mut acc_r = 0u32;
            let mut acc_g = 0u32;
            let mut acc_b = 0u32;
            for dy in 0..s {
                for dx in 0..s {
                    let i = ((y * s + dy) * bw_us + (x * s + dx)) * 4;
                    let pb = raw[i] as u32;
                    let pg = raw[i + 1] as u32;
                    let pr = raw[i + 2] as u32;
                    if pr > 250 && pg < 8 && pb > 250 {
                        continue;
                    }
                    cover += 1;
                    acc_r += pr;
                    acc_g += pg;
                    acc_b += pb;
                }
            }
            if cover == 0 {
                continue;
            }
            let alpha = cover * 255 / samples;
            let o = (y * cw as usize + x) * 4;
            out[o] = (acc_b / cover * alpha / 255) as u8;
            out[o + 1] = (acc_g / cover * alpha / 255) as u8;
            out[o + 2] = (acc_r / cover * alpha / 255) as u8;
            out[o + 3] = alpha as u8;
        }
    }
    let mut out_info: BITMAPINFO = std::mem::zeroed();
    out_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    out_info.bmiHeader.biWidth = cw;
    out_info.bmiHeader.biHeight = -ch;
    out_info.bmiHeader.biPlanes = 1;
    out_info.bmiHeader.biBitCount = 32;
    out_info.bmiHeader.biCompression = BI_RGB as u32;
    let mut bits = null_mut();
    let out_bmp = CreateDIBSection(hdc, &out_info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if out_bmp.is_null() || bits.is_null() {
        if !out_bmp.is_null() {
            DeleteObject(out_bmp as _);
        }
        return;
    }
    std::ptr::copy_nonoverlapping(out.as_ptr(), bits as *mut u8, out.len());
    let out_dc = CreateCompatibleDC(hdc);
    if out_dc.is_null() {
        DeleteObject(out_bmp as _);
        return;
    }
    let old = SelectObject(out_dc, out_bmp as _);
    let mut win_rc: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut win_rc);
    let mut dst = POINT {
        x: win_rc.left,
        y: win_rc.top,
    };
    let mut src = POINT { x: 0, y: 0 };
    let mut size = winapi::shared::windef::SIZE { cx: cw, cy: ch };
    let mut blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA,
    };
    let ok = UpdateLayeredWindow(
        hwnd,
        hdc,
        &mut dst,
        &mut size,
        out_dc,
        &mut src,
        0,
        &mut blend,
        2,
    );
    if ok == 0 {
        log::debug!("KomiRemote pills: UpdateLayeredWindow failed");
    }
    SelectObject(out_dc, old);
    DeleteObject(out_bmp as _);
    DeleteDC(out_dc);
}

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let (cw, ch) = (client.right - client.left, client.bottom - client.top);
    let mem = CreateCompatibleDC(hdc);
    let scale = 4i32;
    let bw = cw.max(1) * scale;
    let bh = ch.max(1) * scale;
    let bmp = winapi::um::wingdi::CreateCompatibleBitmap(hdc, bw, bh);
    let old_bmp = SelectObject(mem, bmp as _);
    let key = CreateSolidBrush(COLOR_KEY);
    let big = RECT {
        left: 0,
        top: 0,
        right: bw,
        bottom: bh,
    };
    FillRect(mem, &big, key);
    DeleteObject(key as _);
    let m0 = metrics(hwnd);
    let m = Metrics {
        btn: m0.btn * scale,
        radius: m0.radius * scale,
        gap: m0.gap * scale,
        pad: m0.pad * scale,
        pill_w: m0.pill_w * scale,
        scale: m0.scale * scale as f32,
    };
    let (items, hover, dark, expanded) = {
        let st = STATE.lock().unwrap();
        (
            st.items.clone(),
            st.hover,
            st.dark,
            expanded_now(&st),
        )
    };
    let (bg, fg, border) = if dark {
        (RGB(22, 24, 28), RGB(240, 243, 245), RGB(58, 64, 72))
    } else {
        (RGB(255, 255, 255), RGB(24, 24, 28), RGB(214, 220, 226))
    };
    let red = RGB(255, 82, 82);
    let red_hover = RGB(198, 40, 40);
    let green = RGB(79, 187, 133);
    let green_hover = RGB(46, 139, 87);
    let text_font = font((13.0 * m.scale) as i32);
    SetBkMode(mem, TRANSPARENT as _);

    let screen = GetDC(null_mut());
    let icons = [
        glyph_dc(screen, 0, m.btn), // display.svg: immediately after actions.svg
        glyph_dc(screen, 1, m.btn), // existing voice_call.svg
        glyph_dc(screen, 2, m.btn), // existing Material videocam path
        glyph_dc(screen, 3, m.btn), // existing call_end.svg
        glyph_dc(screen, 4, m.btn), // session toolbar close.svg
    ];
    if !screen.is_null() {
        ReleaseDC(null_mut(), screen);
    }
    let icon = |kind: Kind, action: bool| -> Option<&GlyphCache> {
        let idx = if action {
            match kind {
                Kind::Call | Kind::VideoCall => 3,
                Kind::Control | Kind::Camera => 4,
            }
        } else {
            match kind {
                Kind::Control => 0,
                Kind::Call => 1,
                Kind::Camera | Kind::VideoCall => 2,
            }
        };
        icons[idx].as_ref()
    };

    if !expanded && !items.is_empty() {
        let (w, h) = window_size(&m, items.len(), false);
        round_fill(mem, 0, 0, w, h, m.radius + m.pad, bg, border);
    }
    for (i, item) in items.iter().enumerate() {
        let hot = hover == Some(i);
        if expanded {
            let top = i as i32 * (row_h(&m) + m.gap);
            round_fill(mem, 0, top, m.pill_w, top + row_h(&m), m.radius + m.pad, bg, border);
            let bx = m.pad;
            let by = top + m.pad;
            round_fill(
                mem,
                bx,
                by,
                bx + m.btn,
                by + m.btn,
                m.radius,
                if hot { green_hover } else { green },
                if hot { green_hover } else { green },
            );
            if let Some(idc) = icon(item.kind, false) {
                blit_glyph(mem, idc, bx, by, m.btn);
            }
            let ax = m.pill_w - m.pad - m.btn;
            round_fill(
                mem,
                ax,
                by,
                ax + m.btn,
                by + m.btn,
                m.radius,
                if hot { red_hover } else { red },
                if hot { red_hover } else { red },
            );
            if let Some(adc) = icon(item.kind, true) {
                blit_glyph(mem, adc, ax, by, m.btn);
            }
            let mut rc = RECT {
                left: bx + m.btn + m.gap,
                top: by,
                right: ax - m.gap,
                bottom: by + m.btn,
            };
            SelectObject(mem, text_font as _);
            SetTextColor(mem, fg);
            draw_text(
                mem,
                &item.text,
                &mut rc,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        } else {
            let bx = m.pad + i as i32 * (m.btn + m.gap);
            let by = m.pad;
            round_fill(
                mem,
                bx,
                by,
                bx + m.btn,
                by + m.btn,
                m.radius,
                if hot { green_hover } else { green },
                if hot { green_hover } else { green },
            );
            if let Some(idc) = icon(item.kind, false) {
                blit_glyph(mem, idc, bx, by, m.btn);
            }
        }
    }

    SelectObject(mem, old_bmp);
    present_smooth(hwnd, hdc, bmp, cw.max(1), ch.max(1), 4);
    DeleteObject(bmp as _);
    DeleteObject(text_font as _);
    DeleteDC(mem);
    for icon in icons.into_iter().flatten() {
        SelectObject(icon.dc, icon.old);
        DeleteObject(icon.bmp as _);
        DeleteDC(icon.dc);
    }
    EndPaint(hwnd, &ps);
}

fn lparam_point(l: LPARAM) -> (i32, i32) {
    (
        (l & 0xffff) as u16 as i16 as i32,
        ((l >> 16) & 0xffff) as u16 as i16 as i32,
    )
}

unsafe fn set_hover(hwnd: HWND, row: Option<usize>) {
    let mut st = STATE.lock().unwrap();
    if st.hover != row {
        st.hover = row;
        drop(st);
        InvalidateRect(hwnd, null_mut(), 0);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            refresh(hwnd);
            0
        }
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as _,
        WM_NCHITTEST => {
            let (sx, sy) = lparam_point(l);
            let mut pt = POINT { x: sx, y: sy };
            ScreenToClient(hwnd, &mut pt);
            let m = metrics(hwnd);
            let st = STATE.lock().unwrap();
            let expanded = expanded_now(&st);
            if item_at(&m, st.items.len(), pt.x, pt.y, expanded).is_some() {
                HTCLIENT as _
            } else {
                HTTRANSPARENT as _
            }
        }
        WM_MOUSEMOVE => {
            if hardware_mouse() {
                let (x, y) = lparam_point(l);
                let drag = STATE.lock().unwrap().drag;
                if let Some((dx, dy)) = drag {
                    let mut pt: POINT = std::mem::zeroed();
                    if GetCursorPos(&mut pt) != 0 {
                        STATE.lock().unwrap().position = Some((pt.x - dx, pt.y - dy));
                    }
                } else {
                    let st = STATE.lock().unwrap();
                    let expanded = expanded_now(&st);
                    let row = item_at(&m_of(hwnd), st.items.len(), x, y, expanded);
                    drop(st);
                    set_hover(hwnd, row);
                }
                refresh(hwnd);
            }
            0
        }
        WM_LBUTTONDOWN => {
            if !hardware_mouse() {
                return 0;
            }
            let (x, y) = lparam_point(l);
            let m = metrics(hwnd);
            let mut st = STATE.lock().unwrap();
            let expanded = expanded_now(&st);
            let row = item_at(&m, st.items.len(), x, y, expanded);
            st.armed = row
                .filter(|r| expanded && in_action(&m, x, y, *r, expanded))
                .and_then(|r| st.items.get(r))
                .map(|item| (item.id, item.kind));
            if st.armed.is_none() && row.is_some() {
                st.drag = Some((x, y));
                SetCapture(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            let (x, y) = lparam_point(l);
            let m = metrics(hwnd);
            let physical = hardware_mouse();
            let item = {
                let mut st = STATE.lock().unwrap();
                if st.drag.take().is_some() {
                    ReleaseCapture();
                }
                let armed = st.armed.take();
                let expanded = expanded_now(&st);
                if physical && expanded {
                    item_at(&m, st.items.len(), x, y, expanded)
                        .filter(|r| in_action(&m, x, y, *r, expanded))
                        .and_then(|r| st.items.get(r))
                        .filter(|item| armed == Some((item.id, item.kind)))
                        .cloned()
                } else {
                    None
                }
            };
            if let Some(item) = item {
                log::info!("KomiRemote pill action for client {}", item.id);
                match item.kind {
                    Kind::Call => ui_cm_interface::close_voice_call(item.id),
                    Kind::VideoCall => ui_cm_interface::komi_video_call_hangup(item.id),
                    Kind::Camera => {
                        let hangup = ui_cm_interface::komi_pill_clients()
                            .iter()
                            .any(|c| c.id == item.id && c.in_video_call && !c.is_view_camera);
                        if hangup {
                            ui_cm_interface::komi_video_call_hangup(item.id);
                        } else {
                            ui_cm_interface::close(item.id);
                        }
                    }
                    Kind::Control => ui_cm_interface::close(item.id),
                }
                refresh(hwnd);
            }
            0
        }
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(null_mut(), IDC_HAND));
            1
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

fn m_of(hwnd: HWND) -> Metrics {
    metrics(hwnd)
}

unsafe fn run() {
    let class = wide("KomiRemotePills");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: null_mut(),
        hIcon: null_mut(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszMenuName: null_mut(),
        lpszClassName: class.as_ptr(),
        hIconSm: null_mut(),
    };
    if RegisterClassExW(&wc) == 0 {
        log::warn!("KomiRemote pills: RegisterClassExW failed");
        return;
    }
    let title = wide("KomiRemote");
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
        class.as_ptr(),
        title.as_ptr(),
        WS_POPUP,
        0,
        0,
        1,
        1,
        null_mut(),
        null_mut(),
        null_mut(),
        null_mut(),
    );
    if hwnd.is_null() {
        log::warn!("KomiRemote pills: CreateWindowExW failed");
        return;
    }
    // Per-pixel alpha (UpdateLayeredWindow). Do not also set a color key:
    // the two layered modes cannot be mixed, and a color key would erase the
    // anti-aliased fringe.
    if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) == 0 {
        log::warn!("KomiRemote pills: capture exclusion unavailable");
    }
    SetTimer(hwnd, TIMER_ID, 500, None);
    refresh(hwnd);
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
