use std::env;
use std::path::PathBuf;

fn main() {
    // system-deps resolves the versioned libvala package (libvala-0.56, ...) and
    // emits the cargo:rustc-link-* directives. The series picked depends on the
    // highest enabled v0_NN feature; see Cargo.toml.
    let deps = system_deps::Config::new()
        .probe()
        .expect("failed to probe libvala via system-deps");
    let lib = deps
        .get_by_name("libvala")
        .expect("system-deps did not return the libvala library");

    // `lib.version` is the full modversion (e.g. 0.56.19); the API series is its
    // major.minor (0.56). Propagate both to dependent crates (the `vala` wrapper)
    // via DEP_VALA_* env vars, courtesy of the `links` key.
    let api_version = api_series(&lib.version);
    println!("cargo:api_version={api_version}");
    println!("cargo:full_version={}", lib.version);

    let clang_args: Vec<String> = lib
        .include_paths
        .iter()
        .map(|path| format!("-I{}", path.display()))
        .collect();

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

/// Reduce a full version (`0.56.19`) to its API series (`0.56`).
fn api_series(version: &str) -> String {
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => version.to_string(),
    }
}
