use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Dxgi::Common::*,
            Gdi::*,
        },
        System::Memory::*,
        UI::WindowsAndMessaging::*,
    },
    core::*,
};
use crate::utils::{debug_print, UnsafeSend};

pub static SHOW_SETTINGS: AtomicBool = AtomicBool::new(false);

static OVERLAY_SETTINGS: LazyLock<Mutex<OverlaySettings>> =
    LazyLock::new(|| Mutex::new(OverlaySettings::default()));

pub fn toggle_settings() {
    let current = SHOW_SETTINGS.load(Ordering::SeqCst);
    SHOW_SETTINGS.store(!current, Ordering::SeqCst);
}

struct OverlaySettings {
    show_fps:       bool,
    show_frametime: bool,
    show_avg:       bool,
    show_low1:      bool,
    show_low01:     bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_fps:       true,
            show_frametime: true,
            show_avg:       true,
            show_low1:      true,
            show_low01:     true,
        }
    }
}

fn calculate_stats(history: &VecDeque<f32>) -> (f32, f32, f32) {
    if history.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut sorted = history.iter().copied().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let average = sorted.iter().sum::<f32>() / sorted.len() as f32;
    let p99  = sorted[((sorted.len() as f32 * 0.99)  as usize).min(sorted.len() - 1)];
    let p999 = sorted[((sorted.len() as f32 * 0.999) as usize).min(sorted.len() - 1)];
    (average, p99, p999)
}

pub fn read_frame_data() -> Option<(f32, f32)> {
    unsafe {
        use windows::core::s;
        let mapping = OpenFileMappingA(FILE_MAP_READ.0, false, s!("GlorpFrameTiming")).ok()?;
        let ptr = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 64);
        if ptr.Value.is_null() { return None; }
        let frame_ns = *(ptr.Value as *const u64);
        let frame_ms = frame_ns as f32 / 1_000_000.0;
        let fps = 1000.0 / frame_ms.max(0.001);
        Some((fps, frame_ms))
    }
}

struct OverlayRenderer {
    hwnd:        HWND,
    d2d_factory: ID2D1Factory,
    dw_factory:  IDWriteFactory,
    rt:          ID2D1DCRenderTarget,
    fmt_large:   IDWriteTextFormat,
    fmt_small:   IDWriteTextFormat,
    brush_white: ID2D1SolidColorBrush,
    brush_green: ID2D1SolidColorBrush,
    brush_yellow:ID2D1SolidColorBrush,
    brush_red:   ID2D1SolidColorBrush,
}

impl OverlayRenderer {
    unsafe fn new(hwnd: HWND) -> Result<Self> {
        let d2d_factory: ID2D1Factory = D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            None,
        )?;

        let dw_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let rt: ID2D1DCRenderTarget = d2d_factory.CreateDCRenderTarget(&rt_props)?;

        let fmt_large = dw_factory.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            18.0,
            w!("en-us"),
        )?;

        let fmt_small = dw_factory.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            w!("en-us"),
        )?;

        let brush_white  = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
        let brush_green  = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 0.0, g: 1.0, b: 0.3, a: 1.0 }, None)?;
        let brush_yellow = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 0.8, b: 0.0, a: 1.0 }, None)?;
        let brush_red    = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 0.2, b: 0.2, a: 1.0 }, None)?;

        Ok(Self {
            hwnd,
            d2d_factory,
            dw_factory,
            rt,
            fmt_large,
            fmt_small,
            brush_white,
            brush_green,
            brush_yellow,
            brush_red,
        })
    }

    unsafe fn render(
        &self,
        fps: f32,
        ft: f32,
        avg: f32,
        p99: f32,
        p999: f32,
        settings: &OverlaySettings,
    ) -> Result<()> {
        let mut rect = RECT::default();
        GetWindowRect(self.hwnd, &mut rect)?;
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        // create a DIB section to paint on
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:        std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:       w,
                biHeight:      -h, // top-down
                biPlanes:      1,
                biBitCount:    32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let old = SelectObject(hdc_mem, hbm);

        // bind DC render target to our hdc
        let bind_rect = RECT { left: 0, top: 0, right: w, bottom: h };
        self.rt.BindDC(hdc_mem, &bind_rect)?;

        self.rt.BeginDraw();
        self.rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

        let mut y = 10.0f32;
        let x = 10.0f32;
        let line_h = 22.0f32;

        if settings.show_fps {
            let fps_brush = if fps >= 60.0 { &self.brush_green }
                else if fps >= 30.0 { &self.brush_yellow }
                else { &self.brush_red };
            let text = format!("FPS:      {:>6.1}", fps);
            self.draw_text(&text, x, y, fps_brush)?;
            y += line_h;
        }
        if settings.show_frametime {
            let text = format!("FT:       {:>5.2} ms", ft);
            self.draw_text(&text, x, y, &self.brush_white)?;
            y += line_h;
        }
        if settings.show_avg {
            let text = format!("Avg FT:   {:>5.2} ms", avg);
            self.draw_text(&text, x, y, &self.brush_white)?;
            y += line_h;
        }
        if settings.show_low1 {
            let text = format!("99th%%:   {:>5.2} ms", p99);
            self.draw_text(&text, x, y, &self.brush_white)?;
            y += line_h;
        }
        if settings.show_low01 {
            let text = format!("99.9th%%: {:>5.2} ms", p999);
            self.draw_text(&text, x, y, &self.brush_white)?;
        }

        self.rt.EndDraw(None, None)?;

        // alpha-blend onto screen via UpdateLayeredWindow
        let pt_dst = POINT { x: rect.left, y: rect.top };
        let sz = SIZE { cx: w, cy: h };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp:             AC_SRC_OVER as u8,
            BlendFlags:          0,
            SourceConstantAlpha: 255,
            AlphaFormat:         AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(
            self.hwnd,
            hdc_screen,
            Some(&pt_dst),
            Some(&sz),
            hdc_mem,
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )?;

        SelectObject(hdc_mem, old);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        Ok(())
    }

    unsafe fn draw_text(&self, text: &str, x: f32, y: f32, brush: &ID2D1SolidColorBrush) -> Result<()> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let layout_rect = D2D_RECT_F {
            left:   x,
            top:    y,
            right:  x + 300.0,
            bottom: y + 30.0,
        };
        self.rt.DrawText(
            &wide,
            &self.fmt_small,
            &layout_rect,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
        Ok(())
    }
}

pub fn spawn_imgui_window(parent_hwnd: HWND) {
    std::panic::set_hook(Box::new(|info| {
        debug_print(format!("[glorp] PANIC: {}", info));
    }));

    let parent = UnsafeSend::new(parent_hwnd);

    std::thread::spawn(move || unsafe {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;

        let hinstance: HINSTANCE = GetModuleHandleW(None).unwrap().into();

        let class_name = w!("glorp_overlay");
        let wc = WNDCLASSW {
            lpfnWndProc:   Some(overlay_wnd_proc),
            hInstance:     hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!("glorp overlay"),
            WS_POPUP | WS_VISIBLE,
            0, 0, 1, 1,
            None, None, Some(hinstance), None,
        ).unwrap();

        // position tracker thread
        let overlay = UnsafeSend::new(hwnd);
        let par     = UnsafeSend::new(*parent);
        std::thread::spawn(move || unsafe {
            loop {
                let mut rect = RECT::default();
                if GetWindowRect(*par, &mut rect).is_ok() {
                    SetWindowPos(
                        *overlay,
                        Some(HWND_TOPMOST),
                        rect.left, rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOACTIVATE,
                    ).ok();
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        });

        let renderer = OverlayRenderer::new(hwnd).unwrap_or_else(|e| {
            debug_print(format!("[glorp] D2D init failed: {:?}", e));
            panic!("D2D init failed");
        });
        debug_print("[glorp] D2D renderer ok");

        let mut ft_history: VecDeque<f32> = VecDeque::with_capacity(100);
        let mut current_fps = 0.0f32;
        let mut current_ft  = 0.0f32;
        let mut last_read   = std::time::Instant::now();

        loop {
            // pump messages
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if last_read.elapsed() >= std::time::Duration::from_millis(250) {
                if let Some((fps, ft)) = read_frame_data() {
                    current_fps = fps;
                    current_ft  = ft;
                    ft_history.push_back(ft);
                    if ft_history.len() > 100 { ft_history.pop_front(); }
                }
                last_read = std::time::Instant::now();
            }

            let (avg, p99, p999) = calculate_stats(&ft_history);
            let s = OVERLAY_SETTINGS.lock().unwrap();

            if let Err(e) = renderer.render(current_fps, current_ft, avg, p99, p999, &s) {
                debug_print(format!("[glorp] render error: {:?}", e));
            }
            drop(s);

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// settings window — plain Win32 dialog
pub fn spawn_settings_window() {
    std::thread::spawn(|| unsafe {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;

        let hinstance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let class_name = w!("glorp_settings");

        let wc = WNDCLASSW {
            lpfnWndProc:   Some(settings_wnd_proc),
            hInstance:     hinstance,
            lpszClassName: class_name,
            hbrBackground: HBRUSH(CreateSolidBrush(COLORREF(0x1a1a1a)).0),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!("glorp | settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            100, 100, 300, 280,
            None, None, Some(hinstance), None,
        ).unwrap();

        // create checkboxes
        let labels = [
            (1001u16, w!("FPS")),
            (1002u16, w!("Frame time (ms)")),
            (1003u16, w!("Avg frame time")),
            (1004u16, w!("99th percentile")),
            (1005u16, w!("99.9th percentile")),
        ];

        let s = OVERLAY_SETTINGS.lock().unwrap();
        let states = [s.show_fps, s.show_frametime, s.show_avg, s.show_low1, s.show_low01];
        drop(s);

        for (i, (id, label)) in labels.iter().enumerate() {
            let cb = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                *label,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_CHECKBOX as u32),
                20,
                40 + i as i32 * 36,
                240,
                28,
                Some(hwnd),
                Some(HMENU(*id as *mut _)),
                Some(hinstance),
                None,
            ).unwrap();
            SendMessageW(cb, BM_SETCHECK, 
                WPARAM(if states[i] { BST_CHECKED.0 as usize } else { 0 }), 
                LPARAM(0));
        }

        SHOW_SETTINGS.store(true, Ordering::Relaxed);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        SHOW_SETTINGS.store(false, Ordering::Relaxed);
    });
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as u16;
            if (1001..=1005).contains(&id) {
                let cb = GetDlgItem(hwnd, id as i32);
                let checked = SendMessageW(cb, BM_GETCHECK, WPARAM(0), LPARAM(0));
                let now_checked = checked.0 == BST_CHECKED.0 as isize;
                // toggle
                SendMessageW(cb, BM_SETCHECK,
                    WPARAM(if now_checked { 0 } else { BST_CHECKED.0 as usize }),
                    LPARAM(0));
                let mut s = OVERLAY_SETTINGS.lock().unwrap();
                match id {
                    1001 => s.show_fps       = !now_checked,
                    1002 => s.show_frametime = !now_checked,
                    1003 => s.show_avg       = !now_checked,
                    1004 => s.show_low1      = !now_checked,
                    1005 => s.show_low01     = !now_checked,
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}