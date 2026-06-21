//! Minimal smoke test: link against libvala and call into it.
fn main() {
    unsafe {
        // vala_get_build_version returns the libvala version string.
        let ptr = vala_sys::vala_get_build_version();
        assert!(!ptr.is_null());
        let s = std::ffi::CStr::from_ptr(ptr);
        println!("libvala build version: {}", s.to_string_lossy());

        // Exercise an object constructor + the GType system.
        let ctx = vala_sys::vala_code_context_new();
        assert!(!ctx.is_null());
        gobject_sys::g_object_unref(ctx as *mut _);
    }
}
