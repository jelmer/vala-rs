//! Exercise the visitor, container getters, and SCIP accessors together: parse
//! Vala, walk the tree, and collect declarations with their source positions --
//! the shape a SCIP indexer needs.

use vala::{
    walk, Class, CodeContext, Method, Namespace, Parser, SourceFile, SourceFileType, Visitor,
    Walker,
};

#[derive(Default)]
struct Collector {
    classes: Vec<String>,
    methods: Vec<(String, i32)>, // (full_name, begin line)
}

impl Visitor for Collector {
    fn visit_class(&mut self, w: &Walker, node: &Class) {
        if let Some(name) = node.name() {
            self.classes.push(name);
        }
        w.walk_children(node);
    }

    fn visit_method(&mut self, w: &Walker, node: &Method) {
        let full = node.full_name().unwrap_or_default();
        let line = node.source_reference().map(|r| r.begin().line).unwrap_or(0);
        self.methods.push((full, line));
        w.walk_children(node);
    }
}

/// Parse `src` and run `f` with the context current (libvala reads the current
/// context during traversal and name resolution).
fn with_parsed(src: &str, f: impl FnOnce(&CodeContext)) {
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let file = SourceFile::new(ctx, SourceFileType::Source, "t.vala", Some(src));
        ctx.add_source_file(&file);
        Parser::new().parse(ctx);
        assert_eq!(ctx.report().errors(), 0, "test source should parse cleanly");
        f(ctx);
    });
}

#[test]
fn visitor_collects_classes_and_methods() {
    let src = "\
namespace Demo {
    public class Widget {
        public void draw () {}
        public int width () { return 1; }
    }
}
";
    with_parsed(src, |ctx| {
        let mut collector = Collector::default();
        walk(&mut collector, &ctx.root());

        assert_eq!(collector.classes, vec!["Widget".to_string()]);

        let names: Vec<&str> = collector.methods.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["Demo.Widget.draw", "Demo.Widget.width"]);

        // draw() is on line 3 (1-based) of the snippet.
        let draw_line = collector
            .methods
            .iter()
            .find(|(n, _)| n == "Demo.Widget.draw")
            .map(|(_, l)| *l);
        assert_eq!(draw_line, Some(3));
    });
}

#[test]
fn container_getters_walk_the_tree() {
    let src = "\
namespace Outer {
    public class A {
        public void m () {}
    }
    public class B {}
}
";
    with_parsed(src, |ctx| {
        let root: Namespace = ctx.root();

        // Find the Outer namespace among the root's children.
        let outer = root
            .namespaces()
            .iter()
            .find(|ns| ns.name().as_deref() == Some("Outer"))
            .expect("Outer namespace");

        let class_names: Vec<String> = outer.classes().iter().filter_map(|c| c.name()).collect();
        assert_eq!(class_names, vec!["A".to_string(), "B".to_string()]);

        let a = outer
            .classes()
            .iter()
            .find(|c| c.name().as_deref() == Some("A"))
            .expect("class A");
        // ObjectTypeSymbol::methods is reached through the Deref chain. libvala
        // synthesises an implicit `.new` creation method alongside `m`.
        let method_names: Vec<String> = a.methods().iter().filter_map(|m| m.name()).collect();
        assert!(
            method_names.contains(&"m".to_string()),
            "expected user method m, got {method_names:?}"
        );
    });
}

#[test]
fn pruning_subtree_skips_children() {
    let src = "\
namespace N {
    public class C {
        public void hidden () {}
    }
}
";
    struct Pruner {
        saw_method: bool,
    }
    impl Visitor for Pruner {
        fn visit_class(&mut self, _w: &Walker, _node: &Class) {
            // Do not recurse: methods inside should not be visited.
        }
        fn visit_method(&mut self, _w: &Walker, _node: &Method) {
            self.saw_method = true;
        }
    }

    with_parsed(src, |ctx| {
        let mut pruner = Pruner { saw_method: false };
        walk(&mut pruner, &ctx.root());
        assert!(!pruner.saw_method, "pruned class body should not be walked");
    });
}
