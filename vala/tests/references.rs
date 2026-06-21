//! Prove the use-site -> definition link works after semantic analysis: the
//! core of any SCIP-style indexer.

use vala::{
    walk, Cast, CodeContext, Expression, MemberAccess, Parser, Profile, SourceFile, SourceFileType,
    Visitor, Walker,
};

struct RefFinder {
    /// (member name, resolved definition full name) for refs in `wanted_file`.
    refs: Vec<(String, Option<String>)>,
    wanted_file: String,
}

impl Visitor for RefFinder {
    fn visit_member_access(&mut self, w: &Walker, node: &MemberAccess) {
        // Only record refs that originate in the file under test, not stdlib.
        let code_node: &vala::CodeNode = node.upcast_ref();
        let in_file = code_node
            .source_reference()
            .map(|r| r.file().filename().unwrap_or_default())
            .map(|f| f.ends_with(&self.wanted_file))
            .unwrap_or(false);

        if in_file {
            let name = node.member_name().unwrap_or_default();
            let expr: &Expression = node.upcast_ref();
            let target = expr.symbol_reference().and_then(|s| s.full_name());
            self.refs.push((name, target));
        }
        w.walk_children(node);
    }
}

#[test]
fn member_access_resolves_to_definition() {
    let src = "\
class Foo {
    public int x;
    public void use_it () {
        this.x = 5;
    }
}
";
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        ctx.set_target_profile(Profile::GObject, true);
        let file = SourceFile::new(ctx, SourceFileType::Source, "refs_test.vala", Some(src));
        ctx.add_source_file(&file);
        Parser::new().parse(ctx);
        ctx.check();
        assert_eq!(ctx.report().errors(), 0, "source should type-check");

        let mut finder = RefFinder {
            refs: Vec::new(),
            wanted_file: "refs_test.vala".to_string(),
        };
        walk(&mut finder, &ctx.root());

        // `this.x` must resolve to the field `Foo.x`.
        let x_ref = finder
            .refs
            .iter()
            .find(|(name, _)| name == "x")
            .expect("a member access named x");
        assert_eq!(x_ref.1.as_deref(), Some("Foo.x"));
    });
}
