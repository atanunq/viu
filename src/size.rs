extern crate libc;

use libc::{c_ushort, STDOUT_FILENO, TIOCGWINSZ};
use libc::ioctl;

#[repr(C)]
struct winsize {
    ws_row: c_ushort,
    ws_col: c_ushort,
    ws_xpixel: c_ushort,
    ws_ypixel: c_ushort,
}

pub fn get_size() -> Result<(isize, isize), &'static str> {
    let w = winsize {ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0};
    let r = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &w) };

    match r {
        0 => Ok((w.ws_col as isize, w.ws_row as isize)),
        _ => Err("Operation Failed"),
    }
}
