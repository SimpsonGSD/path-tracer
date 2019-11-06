use rendy::wsi::winit;


pub fn get_physical_window_size(window: &winit::Window) -> (f64, f64) {
    let dpi_factor = window.get_current_monitor().get_hidpi_factor();
    let window_size = window.get_inner_size().unwrap().to_physical(dpi_factor);
    (window_size.width, window_size.height)
}

#[cfg(target_os = "windows")]
pub fn update_window_framebuffer(window: &winit::Window, 
                                 buffer: &mut Vec<u8>, 
                                 buffer_size: (u32, u32)) {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::GetDC;
    use winit::os::windows::WindowExt;
    use winapi::um::wingdi::{StretchDIBits, DIB_RGB_COLORS, SRCCOPY, BITMAPINFO, BI_RGB, RGBQUAD, BITMAPINFOHEADER};
    use winapi::ctypes::c_void;
    
    let hwnd = window.get_hwnd() as HWND;
    let window_size = get_physical_window_size(&window);

    // Note(SS): Top left is (0,0).

    unsafe {
        let hdc = GetDC(hwnd);
        let bmi_colors = [RGBQUAD {
            rgbBlue: 0, 
            rgbGreen: 0, 
            rgbRed: 0, 
            rgbReserved: 0 
        }];
        let bitmap_header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFO>() as u32,
            biWidth: buffer_size.0 as i32,
            biHeight: buffer_size.1 as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression:  BI_RGB,
            biSizeImage: buffer_size.1 * buffer_size.0 * 3,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0
        };
        let bitmap_info = BITMAPINFO{
            bmiHeader: bitmap_header,
            bmiColors: bmi_colors
        };
        let result = StretchDIBits(hdc,
                      0,
                      0,
                      window_size.0 as i32,
                      window_size.1 as i32,
                      0, 
                      0,
                      buffer_size.0 as i32,
                      buffer_size.1 as i32, 
                      buffer.as_mut_ptr() as *mut c_void,
                      &bitmap_info,
                      DIB_RGB_COLORS,
                      SRCCOPY);
        assert_ne!(result, 0);
    };

}

pub fn update_window_framebuffer_rect(window: &winit::Window, 
                                  buffer: &mut Vec<u8>, 
                                  window_pos: (u32, u32), 
                                  buffer_size: (u32, u32)) {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::GetDC;
    use winit::os::windows::WindowExt;
    use winapi::um::wingdi::{StretchDIBits, DIB_RGB_COLORS, SRCCOPY, BITMAPINFO, BI_RGB, RGBQUAD, BITMAPINFOHEADER};
    use winapi::ctypes::c_void;
    
    let hwnd = window.get_hwnd() as HWND;

    // Note(SS): Top left is (0,0).

    unsafe {
        let hdc = GetDC(hwnd);
        let bmi_colors = [RGBQUAD {
            rgbBlue: 0, 
            rgbGreen: 0, 
            rgbRed: 0, 
            rgbReserved: 0 
        }];
        let bitmap_header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFO>() as u32,
            biWidth: buffer_size.0 as i32,
            biHeight: buffer_size.1 as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression:  BI_RGB,
            biSizeImage: buffer_size.1 * buffer_size.0 * 3,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0
        };
        let bitmap_info = BITMAPINFO{
            bmiHeader: bitmap_header,
            bmiColors: bmi_colors
        };
        let result = StretchDIBits(hdc,
                      window_pos.0 as i32, 
                      window_pos.1 as i32, 
                      buffer_size.0 as i32,
                      buffer_size.1 as i32,
                      0, 
                      0,
                      buffer_size.0 as i32,
                      buffer_size.1 as i32, 
                      buffer.as_mut_ptr() as *mut c_void,
                      &bitmap_info,
                      DIB_RGB_COLORS,
                      SRCCOPY);
        assert_ne!(result, 0);
    };

}