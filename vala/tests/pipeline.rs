//! End-to-end test driving the real libvala pipeline: parse Vala source, run
//! semantic analysis, and walk the resulting code tree.

use vala::{Cast, CodeContext, Parser, SourceFile, SourceFileType};

#[test]
fn parse_valid_source_reports_no_errors() {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let src = "namespace Demo { public int answer () { return 42; } }";
        let file = SourceFile::new(ctx, SourceFileType::Source, "demo.vala", Some(src));
        ctx.add_source_file(&file);

        let parser = Parser::new();
        parser.parse(ctx);

        assert_eq!(
            ctx.report().errors(),
            0,
            "valid source should parse cleanly"
        );
    });
}

#[test]
fn parse_invalid_source_reports_errors() {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        // Missing closing brace / malformed declaration.
        let src = "public void broken ( {{{";
        let file = SourceFile::new(ctx, SourceFileType::Source, "broken.vala", Some(src));
        ctx.add_source_file(&file);

        let parser = Parser::new();
        parser.parse(ctx);

        assert!(ctx.report().has_errors(), "malformed source should error");
    });
}

#[test]
fn root_namespace_is_anonymous() {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let root = ctx.root();
        // The global namespace has no name.
        assert_eq!(root.name(), None);
    });
}

#[test]
fn upcast_and_downcast_round_trip() {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let ns = ctx.root();
        // Namespace -> Symbol (upcast) and back (checked downcast).
        let sym: vala::Symbol = ns.clone().upcast();
        let back: vala::Namespace = sym.downcast().expect("downcast Symbol->Namespace");
        assert_eq!(back.name(), None);
    });
}

#[test]
fn wrong_downcast_returns_err() {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let sym: vala::Symbol = ctx.root().upcast();
        // A namespace is not a Class.
        let result: Result<vala::Class, _> = sym.downcast();
        assert!(result.is_err(), "Namespace must not downcast to Class");
    });
}
