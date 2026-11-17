use super::*;
use std::ffi::{c_char, c_void, CStr};
use std::os::unix::ffi::OsStrExt;

// JNI keeps one handle per game view and calls it exclusively on the UI thread.
#[no_mangle]
unsafe extern "C" fn re_open(path: *const c_char) -> *mut c_void {
    let path = Path::new(std::ffi::OsStr::from_bytes(CStr::from_ptr(path).to_bytes()));
    let world = match fs::read(path) {
        Ok(bytes) => World::decode(&bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Some(World::new()),
        Err(_) => None,
    };
    world.map_or(std::ptr::null_mut(), |world| Box::into_raw(Box::new(world)).cast())
}

#[no_mangle]
unsafe extern "C" fn re_close(handle: *mut c_void) {
    drop(Box::from_raw(handle.cast::<World>()));
}

#[no_mangle]
unsafe extern "C" fn re_save(handle: *mut c_void, path: *const c_char) -> bool {
    let path = Path::new(std::ffi::OsStr::from_bytes(CStr::from_ptr(path).to_bytes()));
    (*handle.cast::<World>()).save(path).is_ok()
}

#[no_mangle]
unsafe extern "C" fn re_step(handle: *mut c_void, axis: i32, jump: bool) {
    (*handle.cast::<World>()).step(axis, jump);
}

#[no_mangle]
unsafe extern "C" fn re_edit(handle: *mut c_void, x: i32, y: i32, place: bool) -> bool {
    (*handle.cast::<World>()).edit(x, y, place)
}

#[no_mangle]
unsafe extern "C" fn re_snapshot(handle: *const c_void, output: *mut u8, length: usize) -> bool {
    if output.is_null() || length < 16 + TILES {
        return false;
    }
    let world = &*handle.cast::<World>();
    let output = std::slice::from_raw_parts_mut(output, 16 + TILES);
    output[0..4].copy_from_slice(&(WIDTH as i32).to_ne_bytes());
    output[4..8].copy_from_slice(&(HEIGHT as i32).to_ne_bytes());
    output[8..12].copy_from_slice(&world.x.to_ne_bytes());
    output[12..16].copy_from_slice(&world.y.to_ne_bytes());
    output[16..].copy_from_slice(&world.tiles);
    true
}
