extern crate nuklear_rust;

extern crate winapi;

#[cfg(feature = "piston_image")]
extern crate image;
#[cfg(feature = "own_window")]
mod own_window;

use nuklear_rust::*;
use nuklear_rust::nuklear_sys as nksys;
use std::{ptr, mem, str, slice, ffi};
use std::os::raw;

pub type FontID = usize;

struct GdiFont {
    nk: nksys::nk_user_font,
    height: i32,
    handle: winapi::shared::windef::HFONT,
    dc: winapi::shared::windef::HDC,
}

impl GdiFont {
    pub unsafe fn new(name: &str, size: i32) -> GdiFont {
        let mut metric = winapi::um::wingdi::TEXTMETRICW {
            tmHeight: 0,
            tmAscent: 0,
            tmDescent: 0,
            tmInternalLeading: 0,
            tmExternalLeading: 0,
            tmAveCharWidth: 0,
            tmMaxCharWidth: 0,
            tmWeight: 0,
            tmOverhang: 0,
            tmDigitizedAspectX: 0,
            tmDigitizedAspectY: 0,
            tmFirstChar: 0,
            tmLastChar: 0,
            tmDefaultChar: 0,
            tmBreakChar: 0,
            tmItalic: 0,
            tmUnderlined: 0,
            tmStruckOut: 0,
            tmPitchAndFamily: 0,
            tmCharSet: 0,
        };
        let handle = winapi::um::wingdi::CreateFontA(size,
                                        0,
                                        0,
                                        0,
                                        winapi::um::wingdi::FW_NORMAL,
                                        winapi::shared::minwindef::FALSE as u32,
                                        winapi::shared::minwindef::FALSE as u32,
                                        winapi::shared::minwindef::FALSE as u32,
                                        winapi::um::wingdi::DEFAULT_CHARSET,
                                        winapi::um::wingdi::OUT_DEFAULT_PRECIS,
                                        winapi::um::wingdi::CLIP_DEFAULT_PRECIS,
                                        winapi::um::wingdi::CLEARTYPE_QUALITY,
                                        winapi::um::wingdi::DEFAULT_PITCH | winapi::um::wingdi::FF_DONTCARE,
                                        name.as_ptr() as *const i8);
        let dc = winapi::um::wingdi::CreateCompatibleDC(ptr::null_mut());

        winapi::um::wingdi::SelectObject(dc, handle as *mut winapi::ctypes::c_void);
        winapi::um::wingdi::GetTextMetricsW(dc, &mut metric);

        GdiFont {
            nk: mem::uninitialized(),
            height: metric.tmHeight,
            handle: handle as winapi::shared::windef::HFONT,
            dc: dc,
        }
    }
}

impl Drop for GdiFont {
    fn drop(&mut self) {
        unsafe {
            winapi::um::wingdi::DeleteObject(self.handle as *mut winapi::ctypes::c_void);
            winapi::um::wingdi::DeleteDC(self.dc);
        }
    }
}

pub struct Drawer {
    bitmap: winapi::shared::windef::HBITMAP,
    window_dc: winapi::shared::windef::HDC,
    memory_dc: winapi::shared::windef::HDC,
    width: i32,
    height: i32,
    fonts: Vec<GdiFont>,

    window: Option<winapi::shared::windef::HWND>,
}

impl Drawer {
    pub fn new(window_dc: winapi::shared::windef::HDC, width: u16, height: u16, window: Option<winapi::shared::windef::HWND>) -> Drawer {
        unsafe {
            let drawer = Drawer {
                bitmap: winapi::um::wingdi::CreateCompatibleBitmap(window_dc, width as i32, height as i32),
                window_dc: window_dc,
                memory_dc: winapi::um::wingdi::CreateCompatibleDC(window_dc),
                width: width as i32,
                height: height as i32,
                fonts: Vec::new(),

                window: window,
            };
            winapi::um::wingdi::SelectObject(drawer.memory_dc, drawer.bitmap as *mut winapi::ctypes::c_void);

            drawer
        }
    }

    pub fn install_statics(&self, context: &mut NkContext) {
        unsafe {
            let context: &mut nksys::nk_context = mem::transmute(context);
            context.clip.copy = Some(nk_gdi_clipbard_copy);
            context.clip.paste = Some(nk_gdi_clipbard_paste);
        }
    }

    pub fn window(&self) -> Option<winapi::shared::windef::HWND> {
        self.window
    }

    pub fn process_events(&mut self, ctx: &mut NkContext) -> bool {
        unsafe {
            let mut msg: winapi::um::winuser::MSG = mem::zeroed();
            ctx.input_begin();

            if winapi::um::winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) <= 0 {
                return false;
            } else {
                winapi::um::winuser::TranslateMessage(&mut msg);
                winapi::um::winuser::DispatchMessageW(&mut msg);
            }

            #[cfg(feature = "own_window")]
            own_window::process_events(self, ctx);

            ctx.input_end();
            return true;
        }
    }

    pub fn new_font(&mut self, name: &str, size: u16) -> FontID {
        self.fonts
            .push(unsafe { GdiFont::new(name, size as i32) });

        let index = self.fonts.len() - 1;
        let gdifont = &mut self.fonts[index];

        unsafe {
            ptr::write(&mut gdifont.nk,
                       nksys::nk_user_font {
                           userdata: nksys::nk_handle_ptr(gdifont as *mut _ as *mut raw::c_void),
                           height: gdifont.height as f32,
                           width: None,
                           query: None,
                           texture: nksys::nk_handle::default(),
                       });

            gdifont.nk.height = gdifont.height as f32;
            gdifont.nk.width = Some(nk_gdifont_get_text_width);
        }

        index as FontID
    }

    pub fn font_by_id(&self, id: FontID) -> Option<&NkUserFont> {
        if self.fonts.len() <= id {
            return None;
        }

        Some(unsafe { ::std::mem::transmute(&self.fonts.get(id).unwrap().nk) })
    }

    #[cfg(feature = "piston_image")]
    pub fn add_image(&mut self, img: &image::DynamicImage) -> NkHandle {
        use image::{Pixel, GenericImage};

        let (w, h) = img.dimensions();

        let hbmp: winapi::shared::windef::HBITMAP;

        let bminfo = winapi::um::wingdi::BITMAPINFO {
            bmiHeader: winapi::um::wingdi::BITMAPINFOHEADER {
                biSize: mem::size_of::<winapi::um::wingdi::BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: h as i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: winapi::um::wingdi::BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: unsafe { mem::zeroed() },
        };

        unsafe {
            let mut pv_image_bits = ptr::null_mut();
            let hdc_screen = winapi::um::winuser::GetDC(ptr::null_mut());
            hbmp = winapi::um::wingdi::CreateDIBSection(hdc_screen,
                                           &bminfo,
                                           winapi::um::wingdi::DIB_RGB_COLORS,
                                           &mut pv_image_bits,
                                           ptr::null_mut(),
                                           0);
            winapi::um::winuser::ReleaseDC(ptr::null_mut(), hdc_screen);
            /*if hbmp.is_null() {
		        return;
		    }*/
            //TODO

            let cb_stride = w as u32 * 4;

            for (x, y, p) in img.pixels() {
                let mut p = p.to_rgba();
                p.data = [p.data[2], p.data[1], p.data[0], p.data[3]];
                *(pv_image_bits.offset(((x * 4) + ((h - y - 1) * cb_stride)) as isize) as *mut [u8; 4]) = p.data;
            }
            /*{
		        // couldn't extract image; delete HBITMAP
		        winapi::um::wingdi::DeleteObject(hbmp as *mut winapi::ctypes::c_void);
		        hbmp = ptr::null_mut();
		        return;
		    }*/
            //TODO
        }

        NkHandle::from_ptr(hbmp as *mut raw::c_void)
    }

    pub fn handle_event(&mut self, ctx: &mut NkContext, wnd: winapi::shared::windef::HWND, msg: winapi::shared::minwindef::UINT, wparam: winapi::shared::minwindef::WPARAM, lparam: winapi::shared::minwindef::LPARAM) -> bool {
        match msg {
            winapi::um::winuser::WM_SIZE => {
                let width = lparam as u16;
                let height = (lparam >> 16) as u16;
                if width as i32 != self.width || height as i32 != self.height {
                    unsafe {
                        winapi::um::wingdi::DeleteObject(self.bitmap as *mut winapi::ctypes::c_void);
                        self.bitmap = winapi::um::wingdi::CreateCompatibleBitmap(self.window_dc, width as i32, height as i32);
                        self.width = width as i32;
                        self.height = height as i32;
                        winapi::um::wingdi::SelectObject(self.memory_dc, self.bitmap as *mut winapi::ctypes::c_void);
                    }
                }
            }
            winapi::um::winuser::WM_PAINT => {
                unsafe {
                    let mut paint: winapi::um::winuser::PAINTSTRUCT = mem::zeroed();
                    let dc = winapi::um::winuser::BeginPaint(wnd, &mut paint);
                    self.blit(dc);
                    winapi::um::winuser::EndPaint(wnd, &paint);
                }
                return true;
            }
            winapi::um::winuser::WM_KEYDOWN |
            winapi::um::winuser::WM_KEYUP |
            winapi::um::winuser::WM_SYSKEYDOWN |
            winapi::um::winuser::WM_SYSKEYUP => {
                let down = ((lparam >> 31) & 1) == 0;
                let ctrl = unsafe { (winapi::um::winuser::GetKeyState(winapi::um::winuser::VK_CONTROL) & (1 << 15)) != 0 };

                match wparam as i32 {
                    winapi::um::winuser::VK_SHIFT |
                    winapi::um::winuser::VK_LSHIFT |
                    winapi::um::winuser::VK_RSHIFT => {
                        ctx.input_key(NkKey::NK_KEY_SHIFT, down);
                        return true;
                    }
                    winapi::um::winuser::VK_DELETE => {
                        ctx.input_key(NkKey::NK_KEY_DEL, down);
                        return true;
                    }
                    winapi::um::winuser::VK_RETURN => {
                        ctx.input_key(NkKey::NK_KEY_ENTER, down);
                        return true;
                    }
                    winapi::um::winuser::VK_TAB => {
                        ctx.input_key(NkKey::NK_KEY_TAB, down);
                        return true;
                    }
                    winapi::um::winuser::VK_LEFT => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_TEXT_WORD_LEFT, down);
                        } else {
                            ctx.input_key(NkKey::NK_KEY_LEFT, down);
                        }
                        return true;
                    }
                    winapi::um::winuser::VK_RIGHT => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_TEXT_WORD_RIGHT, down);
                        } else {
                            ctx.input_key(NkKey::NK_KEY_RIGHT, down);
                        }
                        return true;
                    }
                    winapi::um::winuser::VK_BACK => {
                        ctx.input_key(NkKey::NK_KEY_BACKSPACE, down);
                        return true;
                    }
                    winapi::um::winuser::VK_HOME => {
                        ctx.input_key(NkKey::NK_KEY_TEXT_START, down);
                        ctx.input_key(NkKey::NK_KEY_SCROLL_START, down);
                        return true;
                    }
                    winapi::um::winuser::VK_END => {
                        ctx.input_key(NkKey::NK_KEY_TEXT_END, down);
                        ctx.input_key(NkKey::NK_KEY_SCROLL_END, down);
                        return true;
                    }
                    winapi::um::winuser::VK_NEXT => {
                        ctx.input_key(NkKey::NK_KEY_SCROLL_DOWN, down);
                        return true;
                    }
                    winapi::um::winuser::VK_PRIOR => {
                        ctx.input_key(NkKey::NK_KEY_SCROLL_UP, down);
                        return true;
                    }
                    _ => {}
                }
                match wparam as u8 as char {
                    'C' => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_COPY, down);
                            return true;
                        }
                    }		
                    'V' => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_PASTE, down);
                            return true;
                        }
                    }
                    'X' => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_CUT, down);
                            return true;
                        }
                    }
                    'Z' => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_TEXT_UNDO, down);
                            return true;
                        }
                    }   
                    'R' => {
                        if ctrl {
                            ctx.input_key(NkKey::NK_KEY_TEXT_REDO, down);
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            winapi::um::winuser::WM_CHAR => {
                if wparam >= 32 {
                    unsafe { ctx.input_unicode(::std::char::from_u32_unchecked(wparam as u32)); }
                    return true;
                }
            }
            winapi::um::winuser::WM_LBUTTONDOWN => {
                ctx.input_button(NkButton::NK_BUTTON_LEFT,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 true);
                unsafe {
                    winapi::um::winuser::SetCapture(wnd);
                }
                return true;
            }
            winapi::um::winuser::WM_LBUTTONUP => {
                ctx.input_button(NkButton::NK_BUTTON_LEFT,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 false);
                unsafe {
                    winapi::um::winuser::ReleaseCapture();
                }
                return true;
            }
            winapi::um::winuser::WM_RBUTTONDOWN => {
                ctx.input_button(NkButton::NK_BUTTON_RIGHT,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 true);
                unsafe {
                    winapi::um::winuser::SetCapture(wnd);
                }
                return true;
            }
            winapi::um::winuser::WM_RBUTTONUP => {
                ctx.input_button(NkButton::NK_BUTTON_RIGHT,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 false);
                unsafe {
                    winapi::um::winuser::ReleaseCapture();
                }
                return true;
            }
            winapi::um::winuser::WM_MBUTTONDOWN => {
                ctx.input_button(NkButton::NK_BUTTON_MIDDLE,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 true);
                unsafe {
                    winapi::um::winuser::SetCapture(wnd);
                }
                return true;
            }
            winapi::um::winuser::WM_MBUTTONUP => {
                ctx.input_button(NkButton::NK_BUTTON_MIDDLE,
                                 lparam as u16 as i32,
                                 (lparam >> 16) as u16 as i32,
                                 false);
                unsafe {
                    winapi::um::winuser::ReleaseCapture();
                }
                return true;
            }
            winapi::um::winuser::WM_MOUSEWHEEL => {
                ctx.input_scroll(((wparam >> 16) as u16) as f32 / winapi::um::winuser::WHEEL_DELTA as f32);
                return true;
            }
            winapi::um::winuser::WM_MOUSEMOVE => {
                ctx.input_motion(lparam as u16 as i32, (lparam >> 16) as u16 as i32);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn render(&self, ctx: &mut NkContext, clear: NkColor) {
        unsafe {
            let memory_dc = self.memory_dc;
            winapi::um::wingdi::SelectObject(memory_dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
            winapi::um::wingdi::SelectObject(memory_dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_BRUSH as i32));
            self.clear_dc(memory_dc, clear);

            for cmd in ctx.command_iterator() {
                match cmd.get_type() {
                    NkCommandType::NK_COMMAND_ARC_FILLED => {
                        let a: &NkCommandArcFilled = cmd.as_ref();
                        nk_gdi_fill_arc(memory_dc,
                                        a.cx() as i32,
                                        a.cy() as i32,
                                        a.r() as u32,
                                        a.a()[0],
                                        a.a()[1],
                                        a.color());
                    }
                    NkCommandType::NK_COMMAND_ARC => {
                        let a: &NkCommandArc = cmd.as_ref();
                        nk_gdi_stroke_arc(memory_dc,
                                          a.cx() as i32,
                                          a.cy() as i32,
                                          a.r() as u32,
                                          a.a()[0],
                                          a.a()[1],
                                          a.line_thickness() as i32,
                                          a.color());
                    }
                    NkCommandType::NK_COMMAND_SCISSOR => {
                        let s: &NkCommandScissor = cmd.as_ref();
                        nk_gdi_scissor(memory_dc,
                                       s.x() as f32,
                                       s.y() as f32,
                                       s.w() as f32,
                                       s.h() as f32);
                    }
                    NkCommandType::NK_COMMAND_LINE => {
                        let l: &NkCommandLine = cmd.as_ref();
                        nk_gdi_stroke_line(memory_dc,
                                           l.begin().x as i32,
                                           l.begin().y as i32,
                                           l.end().x as i32,
                                           l.end().y as i32,
                                           l.line_thickness() as i32,
                                           l.color());
                    }
                    NkCommandType::NK_COMMAND_RECT => {
                        let r: &NkCommandRect = cmd.as_ref();
                        nk_gdi_stroke_rect(memory_dc,
                                           r.x() as i32,
                                           r.y() as i32,
                                           r.w() as i32,
                                           r.h() as i32,
                                           r.rounding() as u16 as i32,
                                           r.line_thickness() as i32,
                                           r.color());
                    }
                    NkCommandType::NK_COMMAND_RECT_FILLED => {
                        let r: &NkCommandRectFilled = cmd.as_ref();
                        nk_gdi_fill_rect(memory_dc,
                                         r.x() as i32,
                                         r.y() as i32,
                                         r.w() as i32,
                                         r.h() as i32,
                                         r.rounding() as u16 as i32,
                                         r.color());
                    }
                    NkCommandType::NK_COMMAND_CIRCLE => {
                        let c: &NkCommandCircle = cmd.as_ref();
                        nk_gdi_stroke_circle(memory_dc,
                                             c.x() as i32,
                                             c.y() as i32,
                                             c.w() as i32,
                                             c.h() as i32,
                                             c.line_thickness() as i32,
                                             c.color());
                    }
                    NkCommandType::NK_COMMAND_CIRCLE_FILLED => {
                        let c: &NkCommandCircleFilled = cmd.as_ref();
                        nk_gdi_fill_circle(memory_dc,
                                           c.x() as i32,
                                           c.y() as i32,
                                           c.w() as i32,
                                           c.h() as i32,
                                           c.color());
                    }
                    NkCommandType::NK_COMMAND_TRIANGLE => {
                        let t: &NkCommandTriangle = cmd.as_ref();
                        nk_gdi_stroke_triangle(memory_dc,
                                               t.a().x as i32,
                                               t.a().y as i32,
                                               t.b().x as i32,
                                               t.b().y as i32,
                                               t.c().x as i32,
                                               t.c().y as i32,
                                               t.line_thickness() as i32,
                                               t.color());
                    }
                    NkCommandType::NK_COMMAND_TRIANGLE_FILLED => {
                        let t: &NkCommandTriangleFilled = cmd.as_ref();
                        nk_gdi_fill_triangle(memory_dc,
                                             t.a().x as i32,
                                             t.a().y as i32,
                                             t.b().x as i32,
                                             t.b().y as i32,
                                             t.c().x as i32,
                                             t.c().y as i32,
                                             t.color());
                    }
                    NkCommandType::NK_COMMAND_POLYGON => {
                        let p: &NkCommandPolygon = cmd.as_ref();
                        nk_gdi_stroke_polygon(memory_dc,
                                              p.points().as_ptr(),
                                              p.points().len() as usize,
                                              p.line_thickness() as i32,
                                              p.color());
                    }
                    NkCommandType::NK_COMMAND_POLYGON_FILLED => {
                        let p: &NkCommandPolygonFilled = cmd.as_ref();
                        nk_gdi_fill_polygon(memory_dc,
                                            p.points().as_ptr(),
                                            p.points().len() as usize,
                                            p.color());
                    }
                    NkCommandType::NK_COMMAND_POLYLINE => {
                        let p: &NkCommandPolyline = cmd.as_ref();
                        nk_gdi_stroke_polyline(memory_dc,
                                               p.points().as_ptr(),
                                               p.points().len() as usize,
                                               p.line_thickness() as i32,
                                               p.color());
                    }
                    NkCommandType::NK_COMMAND_TEXT => {
                        let t: &NkCommandText = cmd.as_ref();
                        nk_gdi_draw_text(memory_dc,
                                         t.x() as i32,
                                         t.y() as i32,
                                         t.w() as i32,
                                         t.h() as i32,
                                         t.chars().as_ptr() as *const i8,
                                         t.chars().len() as i32,
                                         (t.font()).userdata_ptr().ptr().unwrap() as *const GdiFont,
                                         t.background(),
                                         t.foreground());
                    }
                    NkCommandType::NK_COMMAND_CURVE => {
                        let q: &NkCommandCurve = cmd.as_ref();
                        nk_gdi_stroke_curve(memory_dc,
                                            q.begin(),
                                            q.ctrl()[0],
                                            q.ctrl()[1],
                                            q.end(),
                                            q.line_thickness() as i32,
                                            q.color());
                    }
                    NkCommandType::NK_COMMAND_IMAGE => {
                        let i: &NkCommandImage = cmd.as_ref();
                        nk_gdi_draw_image(memory_dc,
                                          i.x() as i32,
                                          i.y() as i32,
                                          i.w() as i32,
                                          i.h() as i32,
                                          i.img(),
                                          i.col());
                    }
                    _ => {}
                }
            }
            self.blit(self.window_dc);
            ctx.clear();
        }
    }

    unsafe fn clear_dc(&self, dc: winapi::shared::windef::HDC, col: NkColor) {
        let color = convert_color(col);
        let rect = winapi::shared::windef::RECT {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        };
        winapi::um::wingdi::SetBkColor(dc, color);

        winapi::um::wingdi::ExtTextOutW(dc,
                           0,
                           0,
                           winapi::um::wingdi::ETO_OPAQUE,
                           &rect,
                           ptr::null_mut(),
                           0,
                           ptr::null_mut());
    }

    unsafe fn blit(&self, dc: winapi::shared::windef::HDC) {
        winapi::um::wingdi::BitBlt(dc,
                      0,
                      0,
                      self.width,
                      self.height,
                      self.memory_dc,
                      0,
                      0,
                      winapi::um::wingdi::SRCCOPY);
    }
}

impl Drop for Drawer {
    fn drop(&mut self) {
        unsafe {
            winapi::um::wingdi::DeleteObject(self.memory_dc as *mut winapi::ctypes::c_void);
            winapi::um::wingdi::DeleteObject(self.bitmap as *mut winapi::ctypes::c_void);
        }
    }
}

fn convert_color(c: NkColor) -> winapi::shared::windef::COLORREF {
    c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16)
}

unsafe fn nk_gdi_scissor(dc: winapi::shared::windef::HDC, x: f32, y: f32, w: f32, h: f32) {
    winapi::um::wingdi::SelectClipRgn(dc, ptr::null_mut());
    winapi::um::wingdi::IntersectClipRect(dc,
                             x as i32,
                             y as i32,
                             (x + w + 1.0) as i32,
                             (y + h + 1.0) as i32);
}

unsafe fn nk_gdi_stroke_line(dc: winapi::shared::windef::HDC, x0: i32, y0: i32, x1: i32, y1: i32, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    winapi::um::wingdi::MoveToEx(dc, x0, y0, ptr::null_mut());
    winapi::um::wingdi::LineTo(dc, x1, y1);

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_stroke_rect(dc: winapi::shared::windef::HDC, x: i32, y: i32, w: i32, h: i32, r: i32, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    if r == 0 {
        winapi::um::wingdi::Rectangle(dc, x, y, x + w, y + h);
    } else {
        winapi::um::wingdi::RoundRect(dc, x, y, x + w, y + h, r, r);
    }

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_fill_rect(dc: winapi::shared::windef::HDC, x: i32, y: i32, w: i32, h: i32, r: i32, col: NkColor) {
    let color = convert_color(col);

    if r == 0 {
        let rect = winapi::shared::windef::RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        winapi::um::wingdi::SetBkColor(dc, color);
        winapi::um::wingdi::ExtTextOutW(dc,
                           0,
                           0,
                           winapi::um::wingdi::ETO_OPAQUE,
                           &rect,
                           ptr::null_mut(),
                           0,
                           ptr::null_mut());
    } else {
        winapi::um::wingdi::SetDCPenColor(dc, color);
        winapi::um::wingdi::SetDCBrushColor(dc, color);
        winapi::um::wingdi::RoundRect(dc, x, y, x + w, y + h, r, r);
    }
    winapi::um::wingdi::SetDCBrushColor(dc, color);
}

unsafe fn nk_gdi_fill_triangle(dc: winapi::shared::windef::HDC, x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32, col: NkColor) {
    let color = convert_color(col);
    let points = [winapi::shared::windef::POINT { x: x0, y: y0 }, winapi::shared::windef::POINT { x: x1, y: y1 }, winapi::shared::windef::POINT { x: x2, y: y2 }];

    winapi::um::wingdi::SetDCPenColor(dc, color);
    winapi::um::wingdi::SetDCBrushColor(dc, color);
    winapi::um::wingdi::Polygon(dc, &points[0] as *const winapi::shared::windef::POINT, points.len() as i32);
}

unsafe fn nk_gdi_stroke_triangle(dc: winapi::shared::windef::HDC, x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let points = [winapi::shared::windef::POINT { x: x0, y: y0 }, winapi::shared::windef::POINT { x: x1, y: y1 }, winapi::shared::windef::POINT { x: x2, y: y2 }, winapi::shared::windef::POINT { x: x0, y: y0 }];

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    winapi::um::wingdi::Polyline(dc, &points[0] as *const winapi::shared::windef::POINT, points.len() as i32);

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_fill_polygon(dc: winapi::shared::windef::HDC, pnts: *const NkVec2i, count: usize, col: NkColor) {
    if count < 1 {
        return;
    }

    let mut points = [winapi::shared::windef::POINT { x: 0, y: 0 }; 64];
    let color = convert_color(col);
    winapi::um::wingdi::SetDCBrushColor(dc, color);
    winapi::um::wingdi::SetDCPenColor(dc, color);
    let pnts = slice::from_raw_parts(pnts, count);
    for (i, pnt) in pnts.iter().enumerate() {
        points[i].x = pnt.x as i32;
        points[i].y = pnt.y as i32;
    }
    winapi::um::wingdi::Polygon(dc, &points[0], pnts.len() as i32);
}

unsafe fn nk_gdi_stroke_polygon(dc: winapi::shared::windef::HDC, pnts: *const NkVec2i, count: usize, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    if count > 0 {
        let pnts = slice::from_raw_parts(pnts, count);
        winapi::um::wingdi::MoveToEx(dc, pnts[0].x as i32, pnts[0].y as i32, ptr::null_mut());
        for pnt in pnts.iter().skip(1) {
            winapi::um::wingdi::LineTo(dc, pnt.x as i32, pnt.y as i32);
        }
        winapi::um::wingdi::LineTo(dc, pnts[0].x as i32, pnts[0].y as i32);
    }

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_stroke_polyline(dc: winapi::shared::windef::HDC, pnts: *const NkVec2i, count: usize, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    if count > 0 {
        let pnts = slice::from_raw_parts(pnts, count);
        winapi::um::wingdi::MoveToEx(dc, pnts[0].x as i32, pnts[0].y as i32, ptr::null_mut());
        for pnt in pnts.iter().skip(1) {
            winapi::um::wingdi::LineTo(dc, pnt.x as i32, pnt.y as i32);
        }
    }

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_fill_arc(dc: winapi::shared::windef::HDC, cx: i32, cy: i32, r: u32, a1: f32, a2: f32, color: NkColor) {
    let color = convert_color(color);
    winapi::um::wingdi::SetDCBrushColor(dc, color);
    winapi::um::wingdi::SetDCPenColor(dc, color);
    winapi::um::wingdi::AngleArc(dc, cx, cy, r, a1, a2);
}

unsafe fn nk_gdi_stroke_arc(dc: winapi::shared::windef::HDC, cx: i32, cy: i32, r: u32, a1: f32, a2: f32, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    winapi::um::wingdi::AngleArc(dc, cx, cy, r, a1, a2);

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_fill_circle(dc: winapi::shared::windef::HDC, x: i32, y: i32, w: i32, h: i32, col: NkColor) {
    let color = convert_color(col);
    winapi::um::wingdi::SetDCBrushColor(dc, color);
    winapi::um::wingdi::SetDCPenColor(dc, color);
    winapi::um::wingdi::Ellipse(dc, x, y, x + w, y + h);
}

unsafe fn nk_gdi_stroke_circle(dc: winapi::shared::windef::HDC, x: i32, y: i32, w: i32, h: i32, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    winapi::um::wingdi::Ellipse(dc, x, y, x + w, y + h);

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_stroke_curve(dc: winapi::shared::windef::HDC, p1: NkVec2i, p2: NkVec2i, p3: NkVec2i, p4: NkVec2i, line_thickness: i32, col: NkColor) {
    let color = convert_color(col);
    let p = [winapi::shared::windef::POINT {
                 x: p1.x as i32,
                 y: p1.y as i32,
             },
             winapi::shared::windef::POINT {
                 x: p2.x as i32,
                 y: p2.y as i32,
             },
             winapi::shared::windef::POINT {
                 x: p3.x as i32,
                 y: p3.y as i32,
             },
             winapi::shared::windef::POINT {
                 x: p4.x as i32,
                 y: p4.y as i32,
             }];

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        winapi::um::wingdi::SetDCPenColor(dc, color);
    } else {
        pen = winapi::um::wingdi::CreatePen(winapi::um::wingdi::PS_SOLID as i32, line_thickness, color);
        winapi::um::wingdi::SelectObject(dc, pen as *mut winapi::ctypes::c_void);
    }

    winapi::um::wingdi::PolyBezier(dc, &p[0], p.len() as u32);

    if !pen.is_null() {
        winapi::um::wingdi::SelectObject(dc, winapi::um::wingdi::GetStockObject(winapi::um::wingdi::DC_PEN as i32));
        winapi::um::wingdi::DeleteObject(pen as *mut winapi::ctypes::c_void);
    }
}

unsafe fn nk_gdi_draw_image(dc: winapi::shared::windef::HDC, x: i32, y: i32, w: i32, h: i32, mut img: NkImage, _: NkColor) {
    let mut bitmap: winapi::um::wingdi::BITMAP = mem::zeroed();
    let hdc1 = winapi::um::wingdi::CreateCompatibleDC(ptr::null_mut());
    let h_bitmap = img.ptr();

    winapi::um::wingdi::GetObjectW(h_bitmap as *mut winapi::ctypes::c_void,
                      mem::size_of_val(&bitmap) as i32,
                      &mut bitmap as *mut _ as *mut winapi::ctypes::c_void);

    winapi::um::wingdi::SelectObject(hdc1, h_bitmap as *mut winapi::ctypes::c_void);

    let blendfunc = winapi::um::wingdi::BLENDFUNCTION {
        BlendOp: 0,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: 1,
    };

    winapi::um::wingdi::GdiAlphaBlend(dc,
                         x,
                         y,
                         w,
                         h,
                         hdc1,
                         0,
                         0,
                         bitmap.bmWidth,
                         bitmap.bmHeight,
                         blendfunc);

    winapi::um::wingdi::DeleteDC(hdc1);
}

unsafe fn nk_gdi_draw_text(dc: winapi::shared::windef::HDC, x: i32, y: i32, _: i32, _: i32, text: *const i8, text_len: i32, font: *const GdiFont, cbg: NkColor, cfg: NkColor) {
    let wsize = winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8, 0, text, text_len, ptr::null_mut(), 0);
    let mut wstr = vec![0u16; wsize as usize * mem::size_of::<winapi::ctypes::wchar_t>()];
    winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8, 0, text, text_len, wstr.as_mut_ptr(), wsize);

    winapi::um::wingdi::SetBkColor(dc, convert_color(cbg));
    winapi::um::wingdi::SetTextColor(dc, convert_color(cfg));

    winapi::um::wingdi::SelectObject(dc, (*font).handle as *mut winapi::ctypes::c_void);
    winapi::um::wingdi::ExtTextOutW(dc,
                       x,
                       y,
                       winapi::um::wingdi::ETO_OPAQUE,
                       ptr::null_mut(),
                       wstr.as_mut_ptr(),
                       wsize as u32,
                       ptr::null_mut());
    winapi::um::wingdi::SetDCBrushColor(dc, convert_color(cbg));
}

unsafe extern "C" fn nk_gdifont_get_text_width(handle: nksys::nk_handle, _: f32, text: *const i8, len: i32) -> f32 {
    let font = *handle.ptr.as_ref() as *const GdiFont;
    if font.is_null() || text.is_null() {
        return 0.0;
    }

    let mut size = winapi::shared::windef::SIZE { cx: 0, cy: 0 };
    let wsize = winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8, 0, text, len, ptr::null_mut(), 0);
    let mut wstr: Vec<winapi::ctypes::wchar_t> = vec![0; wsize as usize];
    winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8,
                                  0,
                                  text,
                                  len,
                                  wstr.as_mut_slice() as *mut _ as *mut winapi::ctypes::wchar_t,
                                  wsize);

    if winapi::um::wingdi::GetTextExtentPoint32W((*font).dc,
                                    wstr.as_slice() as *const _ as *const winapi::ctypes::wchar_t,
                                    wsize,
                                    &mut size) > 0 {
        size.cx as f32
    } else {
        -1.0
    }
}

unsafe extern "C" fn nk_gdi_clipbard_paste(_: nksys::nk_handle, edit: *mut nksys::nk_text_edit) {
    if winapi::um::winuser::IsClipboardFormatAvailable(winapi::um::winuser::CF_UNICODETEXT) > 0 && winapi::um::winuser::OpenClipboard(ptr::null_mut()) > 0 {
        let clip = winapi::um::winuser::GetClipboardData(winapi::um::winuser::CF_UNICODETEXT);
        if !clip.is_null() {
            let size = winapi::um::winbase::GlobalSize(clip) - 1;
            if size > 0 {
                let wstr = winapi::um::winbase::GlobalLock(clip);
                if !wstr.is_null() {
                    let size = (size / mem::size_of::<winapi::ctypes::wchar_t>() as winapi::shared::basetsd::SIZE_T) as i32;
                    let utf8size = winapi::um::xxx::WideCharToMultiByte(winapi::um::winnls::CP_UTF8,
                                                                 0,
                                                                 wstr as *const u16,
                                                                 size,
                                                                 ptr::null_mut(),
                                                                 0,
                                                                 ptr::null_mut(),
                                                                 ptr::null_mut());
                    if utf8size > 0 {
                        let mut utf8: Vec<u8> = vec![0; utf8size as usize];
                        winapi::um::xxx::WideCharToMultiByte(winapi::um::winnls::CP_UTF8,
                                                      0,
                                                      wstr as *const u16,
                                                      size,
                                                      utf8.as_mut_ptr() as *mut i8,
                                                      utf8size,
                                                      ptr::null_mut(),
                                                      ptr::null_mut());
                        let edit: &mut NkTextEdit = ::std::mem::transmute(edit);
                        edit.paste(str::from_utf8_unchecked(utf8.as_slice()));
                    }
                    winapi::um::winbase::GlobalUnlock(clip);
                }
            }
        }
        winapi::um::winuser::CloseClipboard();
    }
}

unsafe extern "C" fn nk_gdi_clipbard_copy(_: nksys::nk_handle, text: *const i8, _: i32) {
    if winapi::um::winuser::OpenClipboard(ptr::null_mut()) > 0 {
    	let str_size = ffi::CStr::from_ptr(text).to_bytes().len() as i32;
        let wsize = winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8, 0, text, str_size, ptr::null_mut(), 0);
        if wsize > 0 {
            let mem = winapi::um::winbase::GlobalAlloc(2,
                                            ((wsize + 1) as usize * mem::size_of::<winapi::ctypes::wchar_t>()) as winapi::shared::basetsd::SIZE_T); // 2 = GMEM_MOVEABLE
            if !mem.is_null() {
                let wstr = winapi::um::winbase::GlobalLock(mem);
                if !wstr.is_null() {
                    winapi::um::xxx::MultiByteToWideChar(winapi::um::winnls::CP_UTF8, 0, text, str_size, wstr as *mut u16, wsize);
                    *(wstr.offset((wsize * mem::size_of::<u16>() as i32) as isize) as *mut u8) = 0;
                    winapi::um::winbase::GlobalUnlock(mem);

                    winapi::um::winuser::SetClipboardData(winapi::um::winuser::CF_UNICODETEXT, mem);
                }
            }
        }
        winapi::um::winuser::CloseClipboard();
    }
}

#[cfg(feature = "own_window")]
pub fn bundle<'a>(window_name: &str, width: u16, height: u16, font_name: &str, font_size: u16, allocator: &mut NkAllocator) -> (Drawer, NkContext, FontID) {
    let (hwnd, hdc) = own_window::create_env(window_name, width, height);

    let mut drawer = Drawer::new(hdc, width, height, Some(hwnd));

    let font_id = drawer.new_font(font_name, font_size);
    let mut context = {
        let font = drawer.font_by_id(font_id).unwrap();
        NkContext::new(allocator, &font)
    };
    drawer.install_statics(&mut context);

    (drawer, context, font_id as FontID)
}
