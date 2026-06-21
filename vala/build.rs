//! Generate the safe wrapper type hierarchy from the installed libvala vapi.

use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "build/codegen.rs"]
mod codegen;
#[path = "build/vapi.rs"]
mod vapi;

const VAPI_CANDIDATES: &[&str] = &[
    "/usr/share/vala-0.56/vapi/libvala-0.56.vapi",
    "/usr/share/vala/vapi/libvala-0.56.vapi",
];

fn main() {
    let vapi_path = locate_vapi();
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

fn locate_vapi() -> PathBuf {
    if let Ok(p) = env::var("VALA_VAPI") {
        return PathBuf::from(p);
    }
    // Prefer the vapidir reported by pkg-config so we track the installed series.
    if let Ok(out) = std::process::Command::new("pkg-config")
        .args(["--variable=vapidir", "libvala-0.56"])
        .output()
    {
        if out.status.success() {
            let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !dir.is_empty() {
                let candidate = PathBuf::from(dir).join("libvala-0.56.vapi");
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    for c in VAPI_CANDIDATES {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "could not locate libvala-0.56.vapi; set VALA_VAPI to its path. tried pkg-config vapidir and {VAPI_CANDIDATES:?}"
    );
}
