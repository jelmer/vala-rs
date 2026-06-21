use std::env;
use std::path::PathBuf;

const VALA_PKG: &str = "libvala-0.56";

fn main() {
    let lib = pkg_config::Config::new()
        .probe(VALA_PKG)
        .unwrap_or_else(|e| panic!("failed to find {VALA_PKG} via pkg-config: {e}"));

    // pkg-config already emits the cargo:rustc-link-* directives.

    let mut clang_args = Vec::new();
    for path in &lib.include_paths {
        clang_args.push(format!("-I{}", path.display()));
    }

    let header = lib
        .include_paths
        .iter()
        .map(|p| p.join("vala.h"))
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("vala.h"));

    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed=build.rs");

    // Opaque glib/gobject types we want to reuse from the established -sys crates
    // rather than redefine, so the wrapper crate can interoperate with `glib`.
    // Each is blocklisted here and re-aliased to its glib-sys/gobject-sys
    // definition via a raw line below. glib *primitive* aliases (gint, gchar, ...)
    // are left for bindgen to emit since they are plain aliases of C primitives.
    const GLIB_SYS_TYPES: &[&str] = &[
        "gpointer",
        "gconstpointer",
        "GType",
        "GQuark",
        "GError",
        "GList",
        "GSList",
        "GHashTable",
        "GArray",
        "GPtrArray",
        "GByteArray",
        "GBytes",
        "GThread",
        "GMutex",
        "GCond",
        "GDestroyNotify",
        "GEqualFunc",
        "GCompareFunc",
        "GCompareDataFunc",
        "GHashFunc",
        "GFunc",
    ];
    const GOBJECT_SYS_TYPES: &[&str] = &[
        "GObject",
        "GObjectClass",
        "GValue",
        "GClosure",
        "GParamSpec",
        "GTypeModule",
        "GTypeModuleClass",
        "GTypeInstance",
        "GTypeClass",
        "GTypeInterface",
        "GCallback",
    ];

    let mut builder = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .clang_args(&clang_args)
        // Only emit Vala-specific symbols; glib/gobject come from their own -sys crates.
        .allowlist_function("vala_.*")
        .allowlist_type("Vala.*")
        .allowlist_var("VALA_.*")
        .ctypes_prefix("::std::os::raw")
        .layout_tests(false)
        .generate_comments(false)
        .derive_copy(true)
        .derive_debug(false)
        .prepend_enum_name(false);

    for ty in GLIB_SYS_TYPES {
        builder = builder.blocklist_type(ty);
        builder = builder.raw_line(format!("pub use ::glib_sys::{ty};"));
    }
    for ty in GOBJECT_SYS_TYPES {
        builder = builder.blocklist_type(ty);
        builder = builder.raw_line(format!("pub use ::gobject_sys::{ty};"));
    }

    let bindings = builder
        .generate()
        .expect("failed to generate bindings for libvala");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings.rs");
}
