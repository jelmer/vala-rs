//! A small parser for the subset of the Vala `.vapi` grammar we need: class and
//! interface declarations together with their base types. We deliberately ignore
//! method, property and field bodies -- the safe wrappers are hand-written;
//! codegen only models the type hierarchy.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Class {
    /// Fully qualified Vala name, e.g. `Vala.Genie.Parser`.
    pub vala_name: String,
    /// Direct base class (first item after `:` that is itself a class), if any.
    pub parent: Option<String>,
    /// Implemented interfaces (the remaining items after `:`).
    pub interfaces: Vec<String>,
    pub is_abstract: bool,
    /// True for the gee collection classes (cheader valagee.h); these are
    /// generic and handled separately, not as AST nodes.
    pub is_gee: bool,
    /// Whether the declaration is an `interface` rather than a `class`.
    pub is_interface: bool,
}

impl Class {
    /// snake_case identifier base, e.g. `Vala.Genie.Parser` -> `genie_parser`.
    pub fn snake(&self) -> String {
        to_snake(&self.vala_name)
    }

    /// Rust wrapper type name, e.g. `Vala.Genie.Parser` -> `GenieParser`.
    pub fn rust_name(&self) -> String {
        self.vala_name
            .strip_prefix("Vala.")
            .unwrap_or(&self.vala_name)
            .replace('.', "")
    }

    /// C symbol prefix, e.g. `vala_genie_parser`.
    pub fn c_prefix(&self) -> String {
        format!("vala_{}", self.snake())
    }
}

/// A handful of libvala classes use irregular C symbol names that do not follow
/// the CamelCase->snake_case rule (the `type` prefix is not separated).
fn snake_override(short: &str) -> Option<&'static str> {
    match short {
        "TypeCheck" => Some("typecheck"),
        "TypeParameter" => Some("typeparameter"),
        "TypeSymbol" => Some("typesymbol"),
        _ => None,
    }
}

/// Convert a fully qualified Vala name to the libvala C snake_case convention.
/// `Vala.Genie.Parser` -> `genie_parser`, `Vala.CodeContext` -> `code_context`.
fn to_snake(vala_name: &str) -> String {
    let without_root = vala_name.strip_prefix("Vala.").unwrap_or(vala_name);
    if let Some(o) = snake_override(without_root) {
        return o.to_string();
    }
    let mut out = String::new();
    for (i, part) in without_root.split('.').enumerate() {
        if i > 0 {
            out.push('_');
        }
        camel_to_snake(part, &mut out);
    }
    out
}

fn camel_to_snake(s: &str, out: &mut String) {
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            // Insert a separator before an uppercase letter that starts a new
            // word: not at the very start, and following a lowercase/digit or
            // preceding a lowercase (to split runs like "URL" + "Decoder").
            let prev_lower =
                i > 0 && (chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit());
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase();
            if i > 0 && (prev_lower || next_lower) && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
}

/// Parse the vapi text into an ordered map of class declarations keyed by their
/// fully qualified Vala name.
pub fn parse(src: &str) -> BTreeMap<String, Class> {
    let mut classes = BTreeMap::new();
    // Stack of (namespace, brace-depth at which it was opened).
    let mut namespace_stack: Vec<(String, i32)> = Vec::new();
    let mut last_cheader: Option<String> = None;
    let mut depth: i32 = 0;

    for raw in src.lines() {
        let line = raw.trim();

        if let Some(header) = parse_cheader(line) {
            last_cheader = Some(header);
            continue;
        }

        if let Some(ns) = parse_namespace_open(line) {
            // The namespace opens a brace; record the depth *inside* it.
            depth += brace_delta(line);
            namespace_stack.push((ns, depth));
            continue;
        }

        if let Some(decl) = parse_type_decl(line) {
            let prefix = if namespace_stack.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = namespace_stack.iter().map(|(n, _)| n.as_str()).collect();
                format!("{}.", names.join("."))
            };
            let vala_name = format!("{prefix}{}", decl.name);
            let is_gee = last_cheader.as_deref() == Some("valagee.h");
            classes.insert(
                vala_name.clone(),
                Class {
                    vala_name,
                    parent: decl.parent,
                    interfaces: decl.interfaces,
                    is_abstract: decl.is_abstract,
                    is_gee,
                    is_interface: decl.is_interface,
                },
            );
        }

        // Track brace depth for every other line and pop namespaces whose body
        // brace has now closed.
        depth += brace_delta(line);
        while let Some((_, open_depth)) = namespace_stack.last() {
            if depth < *open_depth {
                namespace_stack.pop();
            } else {
                break;
            }
        }
    }

    classes
}

/// Net change in brace nesting on a line, ignoring braces inside string or
/// character literals (the vapi uses `{` only structurally, but be safe).
fn brace_delta(line: &str) -> i32 {
    let mut delta = 0;
    let mut in_str = false;
    let mut in_char = false;
    let mut prev = '\0';
    for c in line.chars() {
        match c {
            '"' if !in_char && prev != '\\' => in_str = !in_str,
            '\'' if !in_str && prev != '\\' => in_char = !in_char,
            '{' if !in_str && !in_char => delta += 1,
            '}' if !in_str && !in_char => delta -= 1,
            _ => {}
        }
        prev = c;
    }
    delta
}

fn parse_cheader(line: &str) -> Option<String> {
    // [CCode (cheader_filename = "vala.h")]
    let start = line.find("cheader_filename")?;
    let rest = &line[start..];
    let q1 = rest.find('"')?;
    let q2 = rest[q1 + 1..].find('"')? + q1 + 1;
    Some(rest[q1 + 1..q2].to_string())
}

fn parse_namespace_open(line: &str) -> Option<String> {
    let line = line.strip_prefix("public ").unwrap_or(line);
    let rest = line.strip_prefix("namespace ")?;
    let name = rest.trim_end_matches('{').trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// A parsed `class`/`interface` declaration head. Bases are recorded
/// positionally and reclassified into parent/interfaces in a later pass.
struct TypeDecl {
    name: String,
    parent: Option<String>,
    interfaces: Vec<String>,
    is_abstract: bool,
    is_interface: bool,
}

/// Parse a `class`/`interface` declaration line. Generic parameters and
/// base-type generic arguments are stripped.
fn parse_type_decl(line: &str) -> Option<TypeDecl> {
    let mut rest = line;
    for modifier in ["public ", "internal ", "protected ", "private "] {
        rest = rest.strip_prefix(modifier).unwrap_or(rest);
    }
    let is_abstract = rest.starts_with("abstract ");
    rest = rest.strip_prefix("abstract ").unwrap_or(rest);

    let is_interface = rest.starts_with("interface ");
    let keyword = if is_interface { "interface " } else { "class " };
    let rest = rest.strip_prefix(keyword)?;

    // Split off the body / opening brace.
    let decl = rest.split('{').next()?.trim();

    let (head, bases) = match decl.split_once(':') {
        Some((h, b)) => (h.trim(), Some(b.trim())),
        None => (decl, None),
    };

    let name = strip_generics(head).trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut parent = None;
    let mut interfaces = Vec::new();
    if let Some(bases) = bases {
        for base in split_top_level(bases) {
            let base = normalize_base(&base);
            if base.is_empty() {
                continue;
            }
            // The first base may be a class; subsequent ones are interfaces.
            // We resolve class-vs-interface in a second pass against the full
            // set, so just record all bases positionally here.
            if parent.is_none() && interfaces.is_empty() {
                parent = Some(base);
            } else {
                interfaces.push(base);
            }
        }
    }

    Some(TypeDecl {
        name,
        parent,
        interfaces,
        is_abstract,
        is_interface,
    })
}

/// Strip a generic parameter list `<...>` from the end of a type head.
fn strip_generics(s: &str) -> String {
    match s.find('<') {
        Some(idx) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

/// Normalise a base type reference: drop the `Vala.` root prefix is *kept* (we
/// store fully qualified names), strip generic arguments.
fn normalize_base(s: &str) -> String {
    strip_generics(s.trim()).trim().to_string()
}

/// Split a comma-separated list while respecting `<...>` nesting.
fn split_top_level(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '<' => {
                depth += 1;
                cur.push(c);
            }
            '>' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}
