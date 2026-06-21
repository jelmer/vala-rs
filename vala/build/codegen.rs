//! Emit safe wrapper types for the libvala AST/class hierarchy parsed from the
//! `.vapi`. We generate, per fundamental class:
//!
//!   * an owned, ref-counted handle struct,
//!   * `Clone`/`Drop` using the root type's `ref`/`unref`,
//!   * the GLib `Cast`-style upcast `Deref` chain to its parent,
//!   * `AsRef<Parent>` along the whole ancestor chain,
//!   * a `FromGlibPtr`/`as_ptr` style raw-pointer bridge used by the runtime,
//!   * an `unsafe checked downcast` from any ancestor via the runtime GType test.
//!
//! Methods are NOT generated here; they live in hand-written modules. This keeps
//! the generated surface to the mechanical part (the hierarchy) and leaves
//! marshalling decisions to curated code.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::vapi::Class;

/// Root fundamental types that own a `vala_<root>_ref` / `_unref` pair. Every
/// other class is ref-counted through its root ancestor. This list is verified
/// against the generated sys bindings at build time.
const ROOTS: &[&str] = &[
    "AttributeCache",
    "BasicBlock",
    "CodeContext",
    "CodeNode",
    "CodeVisitor",
    "Comment",
    "Genie.Scanner", // GenieScanner -- handled via snake below
    "Iterable",
    "Iterator",
    "Map",
    "MapIterator",
    "MarkupReader",
    "PhiFunction",
    "Report",
    "Scanner",
    "Scope",
    "SourceFile",
    "SourceReference",
    "TargetValue",
    "VersionAttribute",
];

pub struct Model {
    classes: BTreeMap<String, Class>,
    /// Set of fully qualified names that are interfaces.
    interfaces: BTreeSet<String>,
}

impl Model {
    pub fn new(mut classes: BTreeMap<String, Class>) -> Self {
        // Skip the generic gee collection classes; they are not AST nodes and
        // need bespoke generic wrappers.
        classes.retain(|_, c| !c.is_gee);

        let interfaces: BTreeSet<String> = classes
            .values()
            .filter(|c| c.is_interface)
            .map(|c| c.vala_name.clone())
            .collect();

        // Second pass: a recorded "parent" that is actually an interface must be
        // moved into the interface list, and the real parent (if any) recovered.
        let names: BTreeSet<String> = classes.keys().cloned().collect();
        for class in classes.values_mut() {
            reclassify_bases(class, &interfaces, &names);
        }

        Model {
            classes,
            interfaces,
        }
    }

    fn get(&self, name: &str) -> Option<&Class> {
        self.classes.get(name)
    }

    /// Walk parent links to the root fundamental type, returning its snake name
    /// (e.g. `code_node`). Panics if no ref-counted root is found, since that
    /// indicates a class we cannot safely manage.
    fn root_snake(&self, class: &Class) -> Option<String> {
        let mut cur = class;
        loop {
            if is_root(&cur.vala_name) {
                return Some(cur.snake());
            }
            let parent = cur.parent.as_ref()?;
            cur = self.get(parent)?;
        }
    }

    /// The chain of ancestor classes (excluding self), nearest first.
    fn ancestors(&self, class: &Class) -> Vec<&Class> {
        let mut out = Vec::new();
        let mut cur = class;
        while let Some(parent) = cur.parent.as_ref() {
            match self.get(parent) {
                Some(p) => {
                    out.push(p);
                    cur = p;
                }
                None => break,
            }
        }
        out
    }

    pub fn generate(&self) -> String {
        let mut out = String::new();
        out.push_str(GENERATED_HEADER);

        // Interface marker types live in a submodule referenced by the wrappers.
        writeln!(
            out,
            "/// Marker types for libvala interfaces, used by the [`crate::Implements`] bound."
        )
        .unwrap();
        writeln!(out, "pub mod iface {{").unwrap();
        for iface in &self.interfaces {
            if let Some(ic) = self.get(iface) {
                writeln!(
                    out,
                    "    /// Marker type for the `{}` interface.",
                    ic.vala_name
                )
                .unwrap();
                writeln!(out, "    pub enum {} {{}}", ic.rust_name()).unwrap();
            }
        }
        writeln!(out, "}}\n").unwrap();

        // Emit only concrete + abstract *classes* that have a ref-counted root.
        // Interfaces become marker traits handled in the runtime module, not
        // owned handles, so we skip them here.
        let mut emitted = Vec::new();
        for class in self.classes.values() {
            if class.is_interface {
                continue;
            }
            let Some(root) = self.root_snake(class) else {
                // No ref-counted root (e.g. a plain compact class with no
                // lifecycle we model); skip with a note for visibility.
                writeln!(
                    out,
                    "// skipped {}: no ref-counted root type",
                    class.vala_name
                )
                .unwrap();
                continue;
            };
            self.emit_class(&mut out, class, &root);
            emitted.push(class);
        }

        emit_registry(&mut out, &emitted);
        out
    }

    fn emit_class(&self, out: &mut String, class: &Class, root_snake: &str) {
        let rust = class.rust_name();
        let c_prefix = class.c_prefix();
        let ancestors = self.ancestors(class);

        writeln!(out, "ffi_wrapper! {{").unwrap();
        writeln!(out, "    /// Safe wrapper for `{}`.", class.vala_name).unwrap();
        if class.is_abstract {
            writeln!(
                out,
                "    ///\n    /// This is an abstract base type; values are obtained by upcasting a\n    /// concrete subtype, never constructed directly."
            )
            .unwrap();
        }
        writeln!(out, "    pub struct {rust}(ptr: *mut ffi::Vala{rust}) {{").unwrap();
        writeln!(out, "        root_ref: vala_{root_snake}_ref,").unwrap();
        writeln!(out, "        root_unref: vala_{root_snake}_unref,").unwrap();
        writeln!(out, "        get_type: {c_prefix}_get_type,").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();

        // Upcast Deref to the immediate parent (if any), else to GTypeInstance
        // is unnecessary -- a root type derefs to nothing.
        if let Some(parent) = class.parent.as_ref().and_then(|p| self.get(p)) {
            let prust = parent.rust_name();
            writeln!(
                out,
                "impl ::std::ops::Deref for {rust} {{ type Target = {prust};"
            )
            .unwrap();
            writeln!(
                out,
                "    fn deref(&self) -> &Self::Target {{ unsafe {{ &*(self as *const Self as *const {prust}) }} }} }}"
            )
            .unwrap();
        }

        // AsRef along the whole ancestor chain (and to self).
        writeln!(
            out,
            "impl AsRef<{rust}> for {rust} {{ fn as_ref(&self) -> &{rust} {{ self }} }}"
        )
        .unwrap();
        for anc in &ancestors {
            let arust = anc.rust_name();
            writeln!(
                out,
                "impl AsRef<{arust}> for {rust} {{ fn as_ref(&self) -> &{arust} {{ unsafe {{ &*(self as *const Self as *const {arust}) }} }} }}"
            )
            .unwrap();
        }

        // Implement the `IsA` marker for self and every ancestor so generic
        // code can accept "anything that is a CodeNode" etc.
        writeln!(out, "unsafe impl _IsA<{rust}> for {rust} {{}}").unwrap();
        for anc in &ancestors {
            let arust = anc.rust_name();
            writeln!(out, "unsafe impl _IsA<{arust}> for {rust} {{}}").unwrap();
        }

        // Interface markers implemented by this class (and inherited).
        let mut ifaces: BTreeSet<String> = class.interfaces.iter().cloned().collect();
        for anc in &ancestors {
            ifaces.extend(anc.interfaces.iter().cloned());
        }
        for iface in &ifaces {
            if let Some(ic) = self.get(iface) {
                let irust = ic.rust_name();
                writeln!(
                    out,
                    "unsafe impl _Implements<iface::{irust}> for {rust} {{}}"
                )
                .unwrap();
            }
        }

        out.push('\n');
    }
}

/// Reassign a class's bases now that we know the full interface set.
fn reclassify_bases(class: &mut Class, interfaces: &BTreeSet<String>, names: &BTreeSet<String>) {
    let mut all_bases = Vec::new();
    if let Some(p) = class.parent.take() {
        all_bases.push(p);
    }
    all_bases.append(&mut class.interfaces);

    let mut parent = None;
    let mut ifaces = Vec::new();
    for base in all_bases {
        if interfaces.contains(&base) {
            ifaces.push(base);
        } else if names.contains(&base) && parent.is_none() {
            parent = Some(base);
        } else if names.contains(&base) {
            // A second known non-interface base should not happen for Vala
            // (single inheritance); keep it as an interface-like marker.
            ifaces.push(base);
        }
        // Unknown bases (e.g. GLib.Object) are dropped: they live outside the
        // libvala type set we model.
    }
    class.parent = parent;
    class.interfaces = ifaces;
}

fn is_root(vala_name: &str) -> bool {
    let short = vala_name.strip_prefix("Vala.").unwrap_or(vala_name);
    ROOTS.contains(&short)
}

fn emit_registry(out: &mut String, classes: &[&Class]) {
    writeln!(out, "/// Number of wrapper types generated from the vapi.").unwrap();
    writeln!(
        out,
        "pub const GENERATED_TYPE_COUNT: usize = {};",
        classes.len()
    )
    .unwrap();
}

const GENERATED_HEADER: &str = "// @generated by build.rs from the installed libvala vapi -- do not edit.\n\nuse crate::object::{IsA as _IsA, Implements as _Implements};\nuse vala_sys as ffi;\n\n";
