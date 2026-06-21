//! Parse a Vala snippet and report how many errors libvala found.
//!
//! Run with: `cargo run -p vala --example parse`

use vala::{CodeContext, Parser, SourceFile, SourceFileType};

fn main() {
    let source = r#"
        namespace Hello {
            public int main (string[] args) {
                stdout.printf ("Hello, Vala!\n");
                return 0;
            }
        }
    "#;

    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        let file = SourceFile::new(ctx, SourceFileType::Source, "hello.vala", Some(source));
        ctx.add_source_file(&file);

        let parser = Parser::new();
        parser.parse(ctx);

        let report = ctx.report();
        println!("libvala {} parsed the snippet", vala::build_version());
        println!(
            "errors: {}, warnings: {}",
            report.errors(),
            report.warnings()
        );
    });
}
