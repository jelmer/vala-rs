//! Generate the safe wrapper type hierarchy from the installed libvala vapi.

use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "build/codegen.rs"]
mod codegen;
#[path = "build/vapi.rs"]
mod vapi;

fn main() {
    // vala-sys probes libvala and exports its API series (e.g. 0.56) via its
    // `links` metadata. The versioned pkg-config package is libvala-<series>.
    let api_version =
        env::var("DEP_VALA_API_VERSION").expect("DEP_VALA_API_VERSION not set by vala-sys");
    let pkg = format!("libvala-{api_version}");

    // Re-export for the crate's own compilation (DEP_* vars only reach build
    // scripts, not rustc), so lib.rs can read it via env!.
    println!("cargo:rustc-env=VALA_API_VERSION={api_version}");

    let vapi_path = locate_vapi(&pkg);
    println!("cargo:rerun-if-env-changed=VALA_VAPI");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/vapi.rs");
    println!("cargo:rerun-if-changed=build/codegen.rs");
    println!("cargo:rerun-if-changed={}", vapi_path.display());

    let src = fs::read_to_string(&vapi_path)
        .unwrap_or_else(|e| panic!("failed to read vapi at {}: {e}", vapi_path.display()));

    let classes = vapi::parse(&src);
    let model = codegen::Model::new(classes);
    let generated = model.generate();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("generated.rs"), generated).expect("failed to write generated.rs");
}

/// Locate the `<pkg>.vapi` for the probed libvala series.
///
/// `VALA_VAPI` overrides discovery. Otherwise we look in the package's
/// pkg-config `vapidir` and its conventional siblings: on some distributions the
/// vapi lives in the unversioned `vala/vapi` rather than the versioned
/// `vala-<series>/vapi` that vapidir reports.
fn locate_vapi(pkg: &str) -> PathBuf {
    if let Ok(p) = env::var("VALA_VAPI") {
        return PathBuf::from(p);
    }

    let vapi_name = format!("{pkg}.vapi");
    let dirs = vapi_dirs(pkg);
    for dir in &dirs {
        let candidate = dir.join(&vapi_name);
        if candidate.exists() {
            return candidate;
        }
    }

    panic!("could not locate {vapi_name}; set VALA_VAPI to its path (looked in {dirs:?})");
}

/// Candidate directories that may hold the libvala vapi, derived from the
/// package's pkg-config `vapidir`.
fn vapi_dirs(pkg: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(vapidir) = pkg_config_vapidir(pkg) {
        // The unversioned sibling: .../vala-<series>/vapi -> .../vala/vapi.
        if let Some(parent) = vapidir.parent().and_then(|p| p.parent()) {
            dirs.push(parent.join("vala").join("vapi"));
        }
        dirs.push(vapidir);
    }
    dirs
}

/// The `vapidir` pkg-config variable for a package, if pkg-config reports one.
fn pkg_config_vapidir(pkg: &str) -> Option<PathBuf> {
    let out = std::process::Command::new("pkg-config")
        .args(["--variable=vapidir", pkg])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(dir))
    }
}
