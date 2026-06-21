//! Sketch of a SCIP-style indexer: parse and check a Vala file, then emit each
//! declaration and each resolved reference with its source range.
//!
//! Run with: `cargo run -p vala --example index`

use vala::{
    walk_source_file, Cast, Class, CodeContext, Expression, Field, MemberAccess, Method, Parser,
    Profile, SourceFile, SourceFileType, Visitor, Walker,
};

struct Indexer;

impl Indexer {
    fn pos<N: AsRef<vala::CodeNode>>(&self, node: &N) -> Option<(i32, i32)> {
        let b = node.as_ref().source_reference()?.begin();
        Some((b.line, b.column))
    }
}

impl Visitor for Indexer {
    fn visit_class(&mut self, w: &Walker, node: &Class) {
        if let Some((l, c)) = self.pos(node) {
            println!(
                "def class    {:<20} @ {l}:{c}",
                node.name().unwrap_or_default()
            );
        }
        w.walk_children(node);
    }

    fn visit_method(&mut self, w: &Walker, node: &Method) {
        if let Some((l, c)) = self.pos(node) {
            println!(
                "def method   {:<20} @ {l}:{c}",
                node.full_name().unwrap_or_default()
            );
        }
        w.walk_children(node);
    }

    fn visit_field(&mut self, w: &Walker, node: &Field) {
        if let Some((l, c)) = self.pos(node) {
            println!(
                "def field    {:<20} @ {l}:{c}",
                node.full_name().unwrap_or_default()
            );
        }
        w.walk_children(node);
    }

    fn visit_member_access(&mut self, w: &Walker, node: &MemberAccess) {
        if let Some((l, c)) = self.pos(node) {
            let expr: &Expression = node.upcast_ref();
            let target = expr
                .symbol_reference()
                .and_then(|s| s.full_name())
                .unwrap_or_else(|| "<unresolved>".to_string());
            println!(
                "ref {:<24} -> {target:<20} @ {l}:{c}",
                node.member_name().unwrap_or_default()
            );
        }
        w.walk_children(node);
    }
}

fn main() {
    let src = "\
class Counter {
    public int count;
    public void increment () {
        this.count = this.count + 1;
    }
}
";
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        ctx.set_target_profile(Profile::GObject, true);
        let file = SourceFile::new(ctx, SourceFileType::Source, "counter.vala", Some(src));
        ctx.add_source_file(&file);
        Parser::new().parse(ctx);
        ctx.check();
        if ctx.report().has_errors() {
            eprintln!("{} error(s) during analysis", ctx.report().errors());
            return;
        }
        let mut indexer = Indexer;
        walk_source_file(&mut indexer, &file);
    });
}
