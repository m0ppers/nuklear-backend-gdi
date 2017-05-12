extern crate nuklear_rust;

extern crate winapi;
extern crate gdi32;
extern crate kernel32;
extern crate user32;

use std::{ptr, mem, str, slice};
use std::os::raw;

struct GdiFont {
    nk: nuklear_rust::nuklear_sys::nk_user_font,
    height: i32,
    handle: winapi::HFONT,
    dc: winapi::HDC,
}

impl GdiFont {
	pub unsafe fn new(name: &str, size: i32) -> GdiFont {
	    let mut metric = winapi::TEXTMETRICW {
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
	    let handle = gdi32::CreateFontA(size, 0, 0, 0, winapi::FW_NORMAL, winapi::FALSE as u32, winapi::FALSE as u32, winapi::FALSE as u32, winapi::DEFAULT_CHARSET, winapi::OUT_DEFAULT_PRECIS, winapi::CLIP_DEFAULT_PRECIS, winapi::CLEARTYPE_QUALITY, winapi::DEFAULT_PITCH | winapi::FF_DONTCARE, name.as_ptr() as *const i8);
		let dc = gdi32::CreateCompatibleDC(ptr::null_mut());
		    
	    gdi32::SelectObject(dc, handle as *mut raw::c_void);
	    gdi32::GetTextMetricsW(dc, &mut metric);
	    
	    GdiFont {
		    nk: mem::uninitialized(), 
		    height: metric.tmHeight,
		    handle: handle as winapi::HFONT,
		    dc: dc,
	    }
	}
}

impl Drop for GdiFont {
	fn drop(&mut self) {
		unsafe {
			gdi32::DeleteObject(self.handle as *mut raw::c_void);
		    gdi32::DeleteDC(self.dc);
		}
	}
}

pub struct Drawer {
    bitmap: winapi::HBITMAP,
    window_dc: winapi::HDC,
    memory_dc: winapi::HDC,
    width: i32,
    height: i32,
    fonts: Vec<GdiFont>,
}

impl Drawer {
	pub fn new(window_dc: winapi::HDC, width: u16, height: u16) -> Drawer {	    
	    unsafe {
		    let drawer = Drawer {
			    bitmap: gdi32::CreateCompatibleBitmap(window_dc, width as i32, height as i32),
			    window_dc: window_dc,
			    memory_dc: gdi32::CreateCompatibleDC(window_dc),
			    width: width as i32,
			    height: height as i32,
			    fonts: Vec::new(),
			};
		    gdi32::SelectObject(drawer.memory_dc, drawer.bitmap as *mut raw::c_void);
		
		    drawer
	    }
	}
	
	pub fn new_font(&mut self, name: &str, size: u16) -> nuklear_rust::NkUserFont {
		self.fonts.push(unsafe { GdiFont::new(name, size as i32) });
		
		let index = self.fonts.len()-1;
		let mut gdifont = &mut self.fonts[index];
		
		unsafe {
			ptr::write(&mut gdifont.nk, nuklear_rust::nuklear_sys::nk_user_font {
			    userdata: nuklear_rust::nuklear_sys::nk_handle_ptr(gdifont as *mut _ as *mut raw::c_void),
			    height: gdifont.height as f32,
			    width: None,
			    query: None,
			    texture: nuklear_rust::nuklear_sys::nk_handle::default(),	
			});
		
			gdifont.nk.height = gdifont.height as f32;
		    gdifont.nk.width = Some(nk_gdifont_get_text_width);
		
			nuklear_rust::NkUserFont::new(&mut gdifont.nk)
		}
	} 
	
	pub fn handle_event(&mut self, ctx: &mut nuklear_rust::NkContext, wnd: winapi::HWND, msg: winapi::UINT, wparam: winapi::WPARAM, lparam: winapi::LPARAM) -> bool {
	    match msg {
		    winapi::WM_SIZE => {
		        let width = lparam as u16;
		        let height = (lparam >> 2) as u16;
		        if width as i32 != self.width || height as i32 != self.height {
		            unsafe {
		            	gdi32::DeleteObject(self.bitmap as *mut raw::c_void);
			            self.bitmap = gdi32::CreateCompatibleBitmap(self.window_dc, width as i32, height as i32);
			            self.width = width as i32;
			            self.height = height as i32;
			            gdi32::SelectObject(self.memory_dc, self.bitmap as *mut raw::c_void);
		            }
				}
		    },
		    winapi::WM_PAINT => {
		        unsafe {
		        	let mut paint: winapi::PAINTSTRUCT = mem::zeroed();
			        let dc = user32::BeginPaint(wnd, &mut paint);
			        self.blit(dc);
			        user32::EndPaint(wnd, &paint);
		        }
		        return true;
		    },
		    winapi::WM_KEYDOWN | winapi::WM_KEYUP | winapi::WM_SYSKEYDOWN | winapi::WM_SYSKEYUP => {
		        let down = !((lparam >> 31) & 1) > 0;
		        let ctrl = unsafe { user32::GetKeyState(winapi::VK_CONTROL) & (1 << 15) > 0 };
		
		        match wparam as i32 {
			        winapi::VK_SHIFT | winapi::VK_LSHIFT | winapi::VK_RSHIFT => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_SHIFT, down);
			            return true;
			        }
			        winapi::VK_DELETE => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_DEL, down);
			            return true;
			        },
			        winapi::VK_RETURN => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_ENTER, down);
			            return true;
			        },
			        winapi::VK_TAB => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_TAB, down);
			            return true;
			        },
			        winapi::VK_LEFT => {
				        if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_WORD_LEFT, down);
			            } else {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_LEFT, down);
			            }
			            return true;
			        },
			        winapi::VK_RIGHT => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_WORD_RIGHT, down);
			            } else {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_RIGHT, down);
			            }
			            return true;
			        },
			        winapi::VK_BACK => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_BACKSPACE, down);
			            return true;
			        },
			        winapi::VK_HOME => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_START, down);
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_SCROLL_START, down);
			            return true;
			        },
			        winapi::VK_END => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_END, down);
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_SCROLL_END, down);
			            return true;
			        },
			        winapi::VK_NEXT => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_SCROLL_DOWN, down);
			            return true;
			        },
			        winapi::VK_PRIOR => {
			            ctx.input_key(nuklear_rust::NkKey::NK_KEY_SCROLL_UP, down);
			            return true;
			        },
			        _ => {}
			    }
		        match wparam as u8 as char {
		        	'C' => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_COPY, down);
			                return true;
			            }
			        },		
			        'V' => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_PASTE, down);
			                return true;
			            }
			        },
			        'X' => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_CUT, down);
			                return true;
			            }
			        },
			        'Z' => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_UNDO, down);
			                return true;
			            }
			        },   
			        'R' => {
			            if ctrl {
			                ctx.input_key(nuklear_rust::NkKey::NK_KEY_TEXT_REDO, down);
			                return true;
			            }
			        },
			        _ => {}
		        }
		    },
		    winapi::WM_CHAR => {
		        if wparam >= 32 {
		            ctx.input_unicode(wparam as u8 as char);
		            return true;
		        }
		    },
		    winapi::WM_LBUTTONDOWN => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_LEFT, lparam as u16 as i32, (lparam >> 2) as u16 as i32, true);
		        unsafe { user32::SetCapture(wnd); }
		        return true;
		    },
		    winapi::WM_LBUTTONUP => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_LEFT, lparam as u16 as i32, (lparam >> 2) as u16 as i32, false);
		        unsafe { user32::ReleaseCapture(); }
		        return true;
		    },
		    winapi::WM_RBUTTONDOWN => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_RIGHT, lparam as u16 as i32, (lparam >> 2) as u16 as i32, true);
		        unsafe { user32::SetCapture(wnd); }
		        return true;
		    },
		    winapi::WM_RBUTTONUP => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_RIGHT, lparam as u16 as i32, (lparam >> 2) as u16 as i32, false);
		        unsafe { user32::ReleaseCapture(); }
		        return true;
		    },
		    winapi::WM_MBUTTONDOWN => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_MIDDLE, lparam as u16 as i32, (lparam >> 2) as u16 as i32, true);
		        unsafe { user32::SetCapture(wnd); }
		        return true;
			},
		    winapi::WM_MBUTTONUP => {
		        ctx.input_button(nuklear_rust::NkButton::NK_BUTTON_MIDDLE, lparam as u16 as i32, (lparam >> 2) as u16 as i32, false);
		        unsafe { user32::ReleaseCapture(); }
		        return true;
		    },
		    winapi::WM_MOUSEWHEEL => {
		        ctx.input_scroll(((wparam >> 2) as u16) as f32 / winapi::WHEEL_DELTA as f32);
		        return true;
		    },
		    winapi::WM_MOUSEMOVE => {
		        ctx.input_motion(lparam as u16 as i32, (lparam >> 2) as u16 as i32);
		        return true;
		    },
		    _ => {}
	    }
	    false
	}
	
	pub fn render(&self, ctx: &mut nuklear_rust::NkContext, clear: nuklear_rust::NkColor) {
	    unsafe {
	    	let memory_dc = self.memory_dc;
		    gdi32::SelectObject(memory_dc, gdi32::GetStockObject(winapi::DC_PEN));
		    gdi32::SelectObject(memory_dc, gdi32::GetStockObject(winapi::DC_BRUSH));
		    self.clear(memory_dc, clear);
		
			for cmd in ctx.command_iterator() {
		        match cmd.get_type() {
			        nuklear_rust::NkCommandType::NK_COMMAND_SCISSOR => {
			            let s = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_scissor;
			            nk_gdi_scissor(memory_dc, (*s).x as f32, (*s).y as f32, (*s).w as f32, (*s).h as f32);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_LINE => {
			            let l = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_line;
			            nk_gdi_stroke_line(memory_dc, (*l).begin.x as i32, (*l).begin.y as i32, (*l).end.x as i32, (*l).end.y as i32, (*l).line_thickness as i32, (*l).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_RECT => {
			            let r = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_rect;
			            nk_gdi_stroke_rect(memory_dc, (*r).x as i32, (*r).y as i32, (*r).w as i32, (*r).h as i32, (*r).rounding as u16 as i32, (*r).line_thickness as i32, (*r).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_RECT_FILLED => {
			            let r = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_rect_filled;
			            nk_gdi_fill_rect(memory_dc, (*r).x as i32, (*r).y as i32, (*r).w as i32, (*r).h as i32, (*r).rounding as u16 as i32, (*r).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_CIRCLE => {
			            let c = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_circle;
			            nk_gdi_stroke_circle(memory_dc, (*c).x as i32, (*c).y as i32, (*c).w as i32, (*c).h as i32, (*c).line_thickness as i32, (*c).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_CIRCLE_FILLED => {
			            let c = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_circle_filled;
			            nk_gdi_fill_circle(memory_dc, (*c).x as i32, (*c).y as i32, (*c).w as i32, (*c).h as i32, (*c).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_TRIANGLE => {
			            let t = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_triangle;
			            nk_gdi_stroke_triangle(memory_dc, (*t).a.x as i32, (*t).a.y as i32, (*t).b.x as i32, (*t).b.y as i32, (*t).c.x as i32, (*t).c.y as i32, (*t).line_thickness as i32, (*t).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_TRIANGLE_FILLED => {
			            let t = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_triangle_filled;
			            nk_gdi_fill_triangle(memory_dc, (*t).a.x as i32, (*t).a.y as i32, (*t).b.x as i32, (*t).b.y as i32, (*t).c.x as i32, (*t).c.y as i32, (*t).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_POLYGON => {
			            let p = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_polygon;
			            nk_gdi_stroke_polygon(memory_dc, &(*p).points[0], (*p).point_count as usize, (*p).line_thickness as i32, (*p).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_POLYGON_FILLED => {
			            let p = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_polygon_filled;
			            nk_gdi_fill_polygon(memory_dc, &(*p).points[0], (*p).point_count as usize, (*p).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_POLYLINE => {
			            let p = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_polyline;
			            nk_gdi_stroke_polyline(memory_dc, &(*p).points[0], (*p).point_count as usize, (*p).line_thickness as i32, (*p).color);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_TEXT => {
			            let t = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_text;
			            nk_gdi_draw_text(memory_dc, (*t).x as i32, (*t).y as i32, (*t).w as i32, (*t).h as i32, &(*t).string[0], (*t).length, *(*(*t).font).userdata.ptr.as_ref() as *const GdiFont, (*t).background, (*t).foreground);
			        },
			        nuklear_rust::NkCommandType::NK_COMMAND_CURVE => {
			            let q = &cmd as *const _ as *const nuklear_rust::nuklear_sys::nk_command_curve;
			            nk_gdi_stroke_curve(memory_dc, (*q).begin, (*q).ctrl[0], (*q).ctrl[1], (*q).end, (*q).line_thickness as i32, (*q).color);
			        },
			        _ => {}
		        }
		    }
		    self.blit(self.window_dc);
		    ctx.clear();
	    }
	}
	
	unsafe fn clear(&self, dc: winapi::HDC, col: nuklear_rust::NkColor) {
	    let color = convert_color(col);
	    let rect = winapi::RECT { left:0, top:0, right:self.width, bottom:self.height };
	    gdi32::SetBkColor(dc, color);
	
	    gdi32::ExtTextOutW(dc, 0, 0, winapi::ETO_OPAQUE, &rect, ptr::null_mut(), 0, ptr::null_mut());
	}
	
	unsafe fn blit(&self, dc: winapi::HDC) {
	    gdi32::BitBlt(dc, 0, 0, self.width, self.height, self.memory_dc, 0, 0, winapi::SRCCOPY);
	}
}

impl Drop for Drawer {
	fn drop(&mut self) {
		unsafe {
			gdi32::DeleteObject(self.memory_dc as *mut raw::c_void);
		    gdi32::DeleteObject(self.bitmap as *mut raw::c_void);
		}
	}
}

fn convert_color(c: nuklear_rust::NkColor) -> winapi::COLORREF {
    c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16)
}

unsafe fn nk_gdi_scissor(dc: winapi::HDC, x: f32, y: f32, w: f32, h: f32) {
    gdi32::SelectClipRgn(dc, ptr::null_mut());
	gdi32::IntersectClipRect(dc, x as i32, y as i32, (x + w + 1.0) as i32, (y + h + 1.0) as i32);
}

unsafe fn nk_gdi_stroke_line(dc: winapi::HDC, x0: i32, y0: i32, x1: i32, y1: i32, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
	    gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    gdi32::MoveToEx(dc, x0, y0, ptr::null_mut());
    gdi32::LineTo(dc, x1, y1);

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_stroke_rect(dc: winapi::HDC, x: i32, y: i32, w: i32, h: i32, r: i32, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    gdi32::SetDCBrushColor(dc, winapi::OPAQUE as u32);
    if r == 0 {
        gdi32::Rectangle(dc, x, y, x + w, y + h);
    } else {
        gdi32::RoundRect(dc, x, y, x + w, y + h, r, r);
    }

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_fill_rect(dc: winapi::HDC, x: i32, y: i32, w: i32, h: i32, r: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);

    if r == 0 {
        let rect = winapi::RECT{ left: x, top: y, right: x + w, bottom: y + h };
        gdi32::SetBkColor(dc, color);
        gdi32::ExtTextOutW(dc, 0, 0, winapi::ETO_OPAQUE, &rect, ptr::null_mut(), 0, ptr::null_mut());
    } else {
        gdi32::SetDCPenColor(dc, color);
        gdi32::SetDCBrushColor(dc, color);
        gdi32::RoundRect(dc, x, y, x + w, y + h, r, r);
    }
}

unsafe fn nk_gdi_fill_triangle(dc: winapi::HDC, x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let points = [
        winapi::POINT{ x: x0, y: y0 },
        winapi::POINT{ x: x1, y: y1 },
        winapi::POINT{ x: x2, y: y2 },
    ];

    gdi32::SetDCPenColor(dc, color);
    gdi32::SetDCBrushColor(dc, color);
    gdi32::Polygon(dc, &points[0] as *const winapi::POINT, points.len() as i32);
}

unsafe fn nk_gdi_stroke_triangle(dc: winapi::HDC, x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let points = [
        winapi::POINT{ x: x0, y: y0 },
        winapi::POINT{ x: x1, y: y1 },
        winapi::POINT{ x: x2, y: y2 },
        winapi::POINT{ x: x0, y: y0 },
    ];

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    gdi32::Polyline(dc, &points[0] as *const winapi::POINT, points.len() as i32);

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_fill_polygon(dc: winapi::HDC, pnts: *const nuklear_rust::NkVec2i, count: usize, col: nuklear_rust::NkColor) {
	if count < 1 {
		return;
	}
	
    let mut points = [winapi::POINT {x:0,y:0}; 64];
    let color = convert_color(col);
    gdi32::SetDCBrushColor(dc, color);
    gdi32::SetDCPenColor(dc, color);
    let pnts = slice::from_raw_parts(pnts, count);
    for (i, pnt) in pnts.iter().enumerate() {
        points[i].x = pnt.x as i32;
        points[i].y = pnt.y as i32;
    }
    gdi32::Polygon(dc, &points[0], pnts.len() as i32);
}

unsafe fn nk_gdi_stroke_polygon(dc: winapi::HDC, pnts: *const nuklear_rust::NkVec2i, count: usize, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    if count > 0 {
    	let pnts = slice::from_raw_parts(pnts, count);
        gdi32::MoveToEx(dc, pnts[0].x as i32, pnts[0].y as i32, ptr::null_mut());
        for pnt in pnts.iter().skip(1) {
            gdi32::LineTo(dc, pnt.x as i32, pnt.y as i32);
        }
        gdi32::LineTo(dc, pnts[0].x as i32, pnts[0].y as i32);
    }

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_stroke_polyline(dc: winapi::HDC, pnts: *const nuklear_rust::NkVec2i, count: usize, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    if count > 0 {
    	let pnts = slice::from_raw_parts(pnts, count);
        gdi32::MoveToEx(dc, pnts[0].x as i32, pnts[0].y as i32, ptr::null_mut());
        for pnt in pnts.iter().skip(1) {
            gdi32::LineTo(dc, pnt.x as i32, pnt.y as i32);
        }
    }

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_fill_circle(dc: winapi::HDC, x: i32, y: i32, w: i32, h: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    gdi32::SetDCBrushColor(dc, color);
    gdi32::SetDCPenColor(dc, color);
    gdi32::Ellipse(dc, x, y, x + w, y + h);
}

unsafe fn nk_gdi_stroke_circle(dc: winapi::HDC, x: i32, y: i32, w: i32, h: i32, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    gdi32::SetDCBrushColor(dc, winapi::OPAQUE as u32);
    gdi32::Ellipse(dc, x, y, x + w, y + h);

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_stroke_curve(dc: winapi::HDC, p1: nuklear_rust::NkVec2i, p2: nuklear_rust::NkVec2i, p3: nuklear_rust::NkVec2i, p4: nuklear_rust::NkVec2i, line_thickness: i32, col: nuklear_rust::NkColor) {
    let color = convert_color(col);
    let p = [
        winapi::POINT { x: p1.x as i32, y: p1.y as i32 },
        winapi::POINT { x: p2.x as i32, y: p2.y as i32 },
        winapi::POINT { x: p3.x as i32, y: p3.y as i32 },
        winapi::POINT { x: p4.x as i32, y: p4.y as i32 },
    ];

    let mut pen = ptr::null_mut();
    if line_thickness == 1 {
        gdi32::SetDCPenColor(dc, color);
    } else {
        pen = gdi32::CreatePen(winapi::PS_SOLID, line_thickness, color);
        gdi32::SelectObject(dc, pen as *mut raw::c_void);
    }

    gdi32::SetDCBrushColor(dc, winapi::OPAQUE as u32);
    gdi32::PolyBezier(dc, &p[0], p.len() as u32);

    if !pen.is_null() {
        gdi32::SelectObject(dc, gdi32::GetStockObject(winapi::DC_PEN));
        gdi32::DeleteObject(pen as *mut raw::c_void);
    }
}

unsafe fn nk_gdi_draw_text(dc: winapi::HDC, x: i32, y: i32, w: i32, h: i32, text: *const i8, text_len: i32, font: *const GdiFont, cbg: nuklear_rust::NkColor, cfg: nuklear_rust::NkColor) {
    let wsize = kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text, text_len, ptr::null_mut(), 0);
    let mut wstr = Vec::with_capacity(wsize as usize * mem::size_of::<winapi::wchar_t>());
    kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text, text_len, wstr.as_mut_slice()[0], wsize);

    gdi32::SetBkColor(dc, convert_color(cbg));
    gdi32::SetTextColor(dc, convert_color(cfg));

    gdi32::SelectObject(dc, (*font).handle as *mut raw::c_void);
    gdi32::ExtTextOutW(dc, x, y, winapi::ETO_OPAQUE, ptr::null_mut(), wstr.as_slice()[0], wsize as u32, ptr::null_mut());
}

fn nk_gdifont_create(name: &str, size: i32) -> Box<GdiFont> {
    Box::new(unsafe { GdiFont::new(name, size) })
}

unsafe extern "C" fn nk_gdifont_get_text_width(handle: nuklear_rust::nuklear_sys::nk_handle, height: f32, text: *const i8, len: i32) -> f32 {
	let font = *handle.ptr.as_ref() as *const GdiFont;
    if font.is_null() || text.is_null() {
		return 0.0;
    }	
	
    let mut size = winapi::SIZE {
	    cx:0, cy: 0
    };
    let wsize = kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text, len, ptr::null_mut(), 0);
    let mut wstr: Vec<winapi::wchar_t> = Vec::with_capacity(wsize as usize);
    kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text, len, wstr.as_mut_slice() as *mut _ as *mut winapi::wchar_t, wsize);


    if gdi32::GetTextExtentPoint32W((*font).dc, wstr.as_slice() as *const _ as *const winapi::wchar_t, wsize, &mut size) > 0 {
        size.cx as f32
    } else {
	    -1.0
    }
}

unsafe fn nk_gdi_clipbard_paste(edit: &mut nuklear_rust::NkTextEdit) {
    if user32::IsClipboardFormatAvailable(winapi::CF_UNICODETEXT) > 0 && user32::OpenClipboard(ptr::null_mut()) > 0 {
        let mem = user32::GetClipboardData(winapi::CF_UNICODETEXT); 
        if !mem.is_null() {
            let size = kernel32::GlobalSize(mem) - 1;
            if size > 0 {
                let wstr = kernel32::GlobalLock(mem);
                if !wstr.is_null() {
                	let size = (size / mem::size_of::<winapi::wchar_t>() as u64) as i32;
                    let utf8size = kernel32::WideCharToMultiByte(winapi::CP_UTF8, 0, wstr as *const u16, size, ptr::null_mut(), 0, ptr::null_mut(), ptr::null_mut());
                    if utf8size > 0 {
                        let mut utf8: Vec<u8> = Vec::with_capacity(utf8size as usize);
                        kernel32::WideCharToMultiByte(winapi::CP_UTF8, 0, wstr as *const u16, size, utf8.as_mut_ptr() as *mut i8, utf8size, ptr::null_mut(), ptr::null_mut());
                        edit.paste(str::from_utf8_unchecked(utf8.as_slice()));
                    }
                    kernel32::GlobalUnlock(mem); 
                }
            }
        }
        user32::CloseClipboard();
    }
}

unsafe fn nk_gdi_clipbard_copy(text: &str) {
    if user32::OpenClipboard(ptr::null_mut()) > 0 {
        let wsize = kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text.as_ptr() as *const i8, text.len() as i32, ptr::null_mut(), 0);
        if wsize > 0 {
            let mem = kernel32::GlobalAlloc(2, ((wsize + 1) * mem::size_of::<winapi::wchar_t>() as i32) as u64); // 2 = GMEM_MOVEABLE
            if !mem.is_null() {
                let wstr = kernel32::GlobalLock(mem);
                if !wstr.is_null() {
                    kernel32::MultiByteToWideChar(winapi::CP_UTF8, 0, text.as_ptr() as *const i8, text.len() as i32, wstr as *mut u16, wsize);
                    *(wstr.offset(wsize as isize) as *mut u8) = 0;
                    kernel32::GlobalUnlock(mem); 

                    user32::SetClipboardData(winapi::CF_UNICODETEXT, mem); 
                }
            }
        }
        user32::CloseClipboard();
    }
}