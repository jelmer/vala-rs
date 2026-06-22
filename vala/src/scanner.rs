//! Curated methods on [`Scanner`], libvala's lexer.
//!
//! The scanner turns a [`SourceFile`] into a stream of [`Token`]s. Comments are
//! not their own tokens: as it reads a token, libvala stashes any preceding
//! documentation comment, which [`Scanner::pop_comment`] then yields. Note that
//! libvala only retains `/** ... */` doc comments; ordinary `//` and `/* */`
//! comments are discarded by the scanner and cannot be recovered through it.

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::source::SourceLocation;
use crate::{Comment, Scanner, SourceFile};

impl Scanner {
    /// Create a scanner over `file`.
    pub fn new(file: &SourceFile) -> Self {
        unsafe {
            Self::from_raw_full(ffi::vala_scanner_new(file.as_raw()))
                .expect("vala_scanner_new returned null")
        }
    }

    /// Read the next token, advancing the scanner. Returns `None` once
    /// [`TokenType::Eof`] is reached.
    pub fn read_token(&self) -> Option<Token> {
        unsafe {
            let mut begin = std::mem::zeroed::<ffi::ValaSourceLocation>();
            let mut end = std::mem::zeroed::<ffi::ValaSourceLocation>();
            let raw = ffi::vala_scanner_read_token(self.as_raw(), &mut begin, &mut end);
            let token_type = TokenType::from_raw(raw);
            if token_type == TokenType::Eof {
                return None;
            }
            Some(Token {
                token_type,
                begin: SourceLocation::from_ffi(begin),
                end: SourceLocation::from_ffi(end),
            })
        }
    }

    /// Iterate over the file's tokens, stopping at end of file.
    pub fn tokens(&self) -> Tokens<'_> {
        Tokens { scanner: self }
    }

    /// Remove and return the next documentation comment libvala queued while
    /// scanning, or `None` when none is pending.
    ///
    /// libvala attaches a `/** ... */` comment to the token that follows it, so
    /// drain this after each [`read_token`](Scanner::read_token). Only doc
    /// comments are retained; see the module docs.
    pub fn pop_comment(&self) -> Option<Comment> {
        unsafe { Comment::from_raw_full(ffi::vala_scanner_pop_comment(self.as_raw())) }
    }
}

/// Iterator over a [`Scanner`]'s tokens, created by [`Scanner::tokens`].
pub struct Tokens<'a> {
    scanner: &'a Scanner,
}

impl Iterator for Tokens<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        self.scanner.read_token()
    }
}

/// A single lexical token: its kind and the source range it spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// The lexical category of the token.
    pub token_type: TokenType,
    /// Start position (1-based line/column).
    pub begin: SourceLocation,
    /// End position. As elsewhere in libvala the end column is inclusive of the
    /// last character.
    pub end: SourceLocation,
}

/// A Vala lexical token kind, mirroring libvala's `ValaTokenType`.
///
/// Variants span keywords, literals, identifiers, operators and
/// punctuation. The integer values live only in the generated `vala-sys`
/// bindings; the scanner converts from them internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum TokenType {
    None,
    Abstract,
    As,
    Assign,
    AssignAdd,
    AssignBitwiseAnd,
    AssignBitwiseOr,
    AssignBitwiseXor,
    AssignDiv,
    AssignMul,
    AssignPercent,
    AssignShiftLeft,
    AssignSub,
    Async,
    Base,
    BitwiseAnd,
    BitwiseOr,
    Break,
    Carret,
    Case,
    Catch,
    CharacterLiteral,
    Class,
    CloseBrace,
    CloseBracket,
    CloseParens,
    CloseRegexLiteral,
    CloseTemplate,
    Colon,
    Comma,
    Const,
    Construct,
    Continue,
    Default,
    Delegate,
    Delete,
    Div,
    Do,
    DoubleColon,
    Dot,
    Dynamic,
    Ellipsis,
    Else,
    Enum,
    Ensures,
    Errordomain,
    Eof,
    Extern,
    False,
    Finally,
    For,
    Foreach,
    Get,
    Hash,
    Identifier,
    If,
    In,
    Inline,
    IntegerLiteral,
    Interface,
    Internal,
    Interr,
    Is,
    Lambda,
    Lock,
    Minus,
    Namespace,
    New,
    Null,
    Out,
    OpAnd,
    OpCoalescing,
    OpDec,
    OpEq,
    OpGe,
    OpGt,
    OpInc,
    OpLe,
    OpLt,
    OpNe,
    OpNeg,
    OpOr,
    OpPtr,
    OpShiftLeft,
    OpenBrace,
    OpenBracket,
    OpenParens,
    OpenRegexLiteral,
    OpenTemplate,
    Override,
    Owned,
    Params,
    Partial,
    Percent,
    Plus,
    Private,
    Protected,
    Public,
    RealLiteral,
    Ref,
    RegexLiteral,
    Requires,
    Return,
    Sealed,
    Semicolon,
    Set,
    Signal,
    Sizeof,
    Star,
    Static,
    StringLiteral,
    Struct,
    Switch,
    TemplateStringLiteral,
    This,
    Throw,
    Throws,
    Tilde,
    True,
    Try,
    Typeof,
    Unlock,
    Unowned,
    Using,
    Var,
    VerbatimStringLiteral,
    Virtual,
    Void,
    Volatile,
    Weak,
    While,
    With,
    Yield,
}

impl TokenType {
    /// Convert a raw libvala `ValaTokenType`. Unknown values (e.g. from a
    /// newer libvala than this crate was built against) map to
    /// [`TokenType::None`].
    pub(crate) fn from_raw(raw: ffi::ValaTokenType) -> Self {
        match raw {
            ffi::VALA_TOKEN_TYPE_NONE => TokenType::None,
            ffi::VALA_TOKEN_TYPE_ABSTRACT => TokenType::Abstract,
            ffi::VALA_TOKEN_TYPE_AS => TokenType::As,
            ffi::VALA_TOKEN_TYPE_ASSIGN => TokenType::Assign,
            ffi::VALA_TOKEN_TYPE_ASSIGN_ADD => TokenType::AssignAdd,
            ffi::VALA_TOKEN_TYPE_ASSIGN_BITWISE_AND => TokenType::AssignBitwiseAnd,
            ffi::VALA_TOKEN_TYPE_ASSIGN_BITWISE_OR => TokenType::AssignBitwiseOr,
            ffi::VALA_TOKEN_TYPE_ASSIGN_BITWISE_XOR => TokenType::AssignBitwiseXor,
            ffi::VALA_TOKEN_TYPE_ASSIGN_DIV => TokenType::AssignDiv,
            ffi::VALA_TOKEN_TYPE_ASSIGN_MUL => TokenType::AssignMul,
            ffi::VALA_TOKEN_TYPE_ASSIGN_PERCENT => TokenType::AssignPercent,
            ffi::VALA_TOKEN_TYPE_ASSIGN_SHIFT_LEFT => TokenType::AssignShiftLeft,
            ffi::VALA_TOKEN_TYPE_ASSIGN_SUB => TokenType::AssignSub,
            ffi::VALA_TOKEN_TYPE_ASYNC => TokenType::Async,
            ffi::VALA_TOKEN_TYPE_BASE => TokenType::Base,
            ffi::VALA_TOKEN_TYPE_BITWISE_AND => TokenType::BitwiseAnd,
            ffi::VALA_TOKEN_TYPE_BITWISE_OR => TokenType::BitwiseOr,
            ffi::VALA_TOKEN_TYPE_BREAK => TokenType::Break,
            ffi::VALA_TOKEN_TYPE_CARRET => TokenType::Carret,
            ffi::VALA_TOKEN_TYPE_CASE => TokenType::Case,
            ffi::VALA_TOKEN_TYPE_CATCH => TokenType::Catch,
            ffi::VALA_TOKEN_TYPE_CHARACTER_LITERAL => TokenType::CharacterLiteral,
            ffi::VALA_TOKEN_TYPE_CLASS => TokenType::Class,
            ffi::VALA_TOKEN_TYPE_CLOSE_BRACE => TokenType::CloseBrace,
            ffi::VALA_TOKEN_TYPE_CLOSE_BRACKET => TokenType::CloseBracket,
            ffi::VALA_TOKEN_TYPE_CLOSE_PARENS => TokenType::CloseParens,
            ffi::VALA_TOKEN_TYPE_CLOSE_REGEX_LITERAL => TokenType::CloseRegexLiteral,
            ffi::VALA_TOKEN_TYPE_CLOSE_TEMPLATE => TokenType::CloseTemplate,
            ffi::VALA_TOKEN_TYPE_COLON => TokenType::Colon,
            ffi::VALA_TOKEN_TYPE_COMMA => TokenType::Comma,
            ffi::VALA_TOKEN_TYPE_CONST => TokenType::Const,
            ffi::VALA_TOKEN_TYPE_CONSTRUCT => TokenType::Construct,
            ffi::VALA_TOKEN_TYPE_CONTINUE => TokenType::Continue,
            ffi::VALA_TOKEN_TYPE_DEFAULT => TokenType::Default,
            ffi::VALA_TOKEN_TYPE_DELEGATE => TokenType::Delegate,
            ffi::VALA_TOKEN_TYPE_DELETE => TokenType::Delete,
            ffi::VALA_TOKEN_TYPE_DIV => TokenType::Div,
            ffi::VALA_TOKEN_TYPE_DO => TokenType::Do,
            ffi::VALA_TOKEN_TYPE_DOUBLE_COLON => TokenType::DoubleColon,
            ffi::VALA_TOKEN_TYPE_DOT => TokenType::Dot,
            ffi::VALA_TOKEN_TYPE_DYNAMIC => TokenType::Dynamic,
            ffi::VALA_TOKEN_TYPE_ELLIPSIS => TokenType::Ellipsis,
            ffi::VALA_TOKEN_TYPE_ELSE => TokenType::Else,
            ffi::VALA_TOKEN_TYPE_ENUM => TokenType::Enum,
            ffi::VALA_TOKEN_TYPE_ENSURES => TokenType::Ensures,
            ffi::VALA_TOKEN_TYPE_ERRORDOMAIN => TokenType::Errordomain,
            ffi::VALA_TOKEN_TYPE_EOF => TokenType::Eof,
            ffi::VALA_TOKEN_TYPE_EXTERN => TokenType::Extern,
            ffi::VALA_TOKEN_TYPE_FALSE => TokenType::False,
            ffi::VALA_TOKEN_TYPE_FINALLY => TokenType::Finally,
            ffi::VALA_TOKEN_TYPE_FOR => TokenType::For,
            ffi::VALA_TOKEN_TYPE_FOREACH => TokenType::Foreach,
            ffi::VALA_TOKEN_TYPE_GET => TokenType::Get,
            ffi::VALA_TOKEN_TYPE_HASH => TokenType::Hash,
            ffi::VALA_TOKEN_TYPE_IDENTIFIER => TokenType::Identifier,
            ffi::VALA_TOKEN_TYPE_IF => TokenType::If,
            ffi::VALA_TOKEN_TYPE_IN => TokenType::In,
            ffi::VALA_TOKEN_TYPE_INLINE => TokenType::Inline,
            ffi::VALA_TOKEN_TYPE_INTEGER_LITERAL => TokenType::IntegerLiteral,
            ffi::VALA_TOKEN_TYPE_INTERFACE => TokenType::Interface,
            ffi::VALA_TOKEN_TYPE_INTERNAL => TokenType::Internal,
            ffi::VALA_TOKEN_TYPE_INTERR => TokenType::Interr,
            ffi::VALA_TOKEN_TYPE_IS => TokenType::Is,
            ffi::VALA_TOKEN_TYPE_LAMBDA => TokenType::Lambda,
            ffi::VALA_TOKEN_TYPE_LOCK => TokenType::Lock,
            ffi::VALA_TOKEN_TYPE_MINUS => TokenType::Minus,
            ffi::VALA_TOKEN_TYPE_NAMESPACE => TokenType::Namespace,
            ffi::VALA_TOKEN_TYPE_NEW => TokenType::New,
            ffi::VALA_TOKEN_TYPE_NULL => TokenType::Null,
            ffi::VALA_TOKEN_TYPE_OUT => TokenType::Out,
            ffi::VALA_TOKEN_TYPE_OP_AND => TokenType::OpAnd,
            ffi::VALA_TOKEN_TYPE_OP_COALESCING => TokenType::OpCoalescing,
            ffi::VALA_TOKEN_TYPE_OP_DEC => TokenType::OpDec,
            ffi::VALA_TOKEN_TYPE_OP_EQ => TokenType::OpEq,
            ffi::VALA_TOKEN_TYPE_OP_GE => TokenType::OpGe,
            ffi::VALA_TOKEN_TYPE_OP_GT => TokenType::OpGt,
            ffi::VALA_TOKEN_TYPE_OP_INC => TokenType::OpInc,
            ffi::VALA_TOKEN_TYPE_OP_LE => TokenType::OpLe,
            ffi::VALA_TOKEN_TYPE_OP_LT => TokenType::OpLt,
            ffi::VALA_TOKEN_TYPE_OP_NE => TokenType::OpNe,
            ffi::VALA_TOKEN_TYPE_OP_NEG => TokenType::OpNeg,
            ffi::VALA_TOKEN_TYPE_OP_OR => TokenType::OpOr,
            ffi::VALA_TOKEN_TYPE_OP_PTR => TokenType::OpPtr,
            ffi::VALA_TOKEN_TYPE_OP_SHIFT_LEFT => TokenType::OpShiftLeft,
            ffi::VALA_TOKEN_TYPE_OPEN_BRACE => TokenType::OpenBrace,
            ffi::VALA_TOKEN_TYPE_OPEN_BRACKET => TokenType::OpenBracket,
            ffi::VALA_TOKEN_TYPE_OPEN_PARENS => TokenType::OpenParens,
            ffi::VALA_TOKEN_TYPE_OPEN_REGEX_LITERAL => TokenType::OpenRegexLiteral,
            ffi::VALA_TOKEN_TYPE_OPEN_TEMPLATE => TokenType::OpenTemplate,
            ffi::VALA_TOKEN_TYPE_OVERRIDE => TokenType::Override,
            ffi::VALA_TOKEN_TYPE_OWNED => TokenType::Owned,
            ffi::VALA_TOKEN_TYPE_PARAMS => TokenType::Params,
            ffi::VALA_TOKEN_TYPE_PARTIAL => TokenType::Partial,
            ffi::VALA_TOKEN_TYPE_PERCENT => TokenType::Percent,
            ffi::VALA_TOKEN_TYPE_PLUS => TokenType::Plus,
            ffi::VALA_TOKEN_TYPE_PRIVATE => TokenType::Private,
            ffi::VALA_TOKEN_TYPE_PROTECTED => TokenType::Protected,
            ffi::VALA_TOKEN_TYPE_PUBLIC => TokenType::Public,
            ffi::VALA_TOKEN_TYPE_REAL_LITERAL => TokenType::RealLiteral,
            ffi::VALA_TOKEN_TYPE_REF => TokenType::Ref,
            ffi::VALA_TOKEN_TYPE_REGEX_LITERAL => TokenType::RegexLiteral,
            ffi::VALA_TOKEN_TYPE_REQUIRES => TokenType::Requires,
            ffi::VALA_TOKEN_TYPE_RETURN => TokenType::Return,
            ffi::VALA_TOKEN_TYPE_SEALED => TokenType::Sealed,
            ffi::VALA_TOKEN_TYPE_SEMICOLON => TokenType::Semicolon,
            ffi::VALA_TOKEN_TYPE_SET => TokenType::Set,
            ffi::VALA_TOKEN_TYPE_SIGNAL => TokenType::Signal,
            ffi::VALA_TOKEN_TYPE_SIZEOF => TokenType::Sizeof,
            ffi::VALA_TOKEN_TYPE_STAR => TokenType::Star,
            ffi::VALA_TOKEN_TYPE_STATIC => TokenType::Static,
            ffi::VALA_TOKEN_TYPE_STRING_LITERAL => TokenType::StringLiteral,
            ffi::VALA_TOKEN_TYPE_STRUCT => TokenType::Struct,
            ffi::VALA_TOKEN_TYPE_SWITCH => TokenType::Switch,
            ffi::VALA_TOKEN_TYPE_TEMPLATE_STRING_LITERAL => TokenType::TemplateStringLiteral,
            ffi::VALA_TOKEN_TYPE_THIS => TokenType::This,
            ffi::VALA_TOKEN_TYPE_THROW => TokenType::Throw,
            ffi::VALA_TOKEN_TYPE_THROWS => TokenType::Throws,
            ffi::VALA_TOKEN_TYPE_TILDE => TokenType::Tilde,
            ffi::VALA_TOKEN_TYPE_TRUE => TokenType::True,
            ffi::VALA_TOKEN_TYPE_TRY => TokenType::Try,
            ffi::VALA_TOKEN_TYPE_TYPEOF => TokenType::Typeof,
            ffi::VALA_TOKEN_TYPE_UNLOCK => TokenType::Unlock,
            ffi::VALA_TOKEN_TYPE_UNOWNED => TokenType::Unowned,
            ffi::VALA_TOKEN_TYPE_USING => TokenType::Using,
            ffi::VALA_TOKEN_TYPE_VAR => TokenType::Var,
            ffi::VALA_TOKEN_TYPE_VERBATIM_STRING_LITERAL => TokenType::VerbatimStringLiteral,
            ffi::VALA_TOKEN_TYPE_VIRTUAL => TokenType::Virtual,
            ffi::VALA_TOKEN_TYPE_VOID => TokenType::Void,
            ffi::VALA_TOKEN_TYPE_VOLATILE => TokenType::Volatile,
            ffi::VALA_TOKEN_TYPE_WEAK => TokenType::Weak,
            ffi::VALA_TOKEN_TYPE_WHILE => TokenType::While,
            ffi::VALA_TOKEN_TYPE_WITH => TokenType::With,
            ffi::VALA_TOKEN_TYPE_YIELD => TokenType::Yield,
            _ => TokenType::None,
        }
    }
}

impl Comment {
    /// The source range this comment spans.
    pub fn source_reference(&self) -> Option<crate::SourceReference> {
        unsafe {
            crate::SourceReference::from_raw_none(ffi::vala_comment_get_source_reference(
                self.as_raw(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CodeContext, SourceFileType};

    fn token_types(source: &str) -> Vec<TokenType> {
        let ctx = CodeContext::new();
        ctx.with_current(|ctx| {
            let file = SourceFile::new(ctx, SourceFileType::Source, "test.vala", Some(source));
            ctx.add_source_file(&file);
            Scanner::new(&file).tokens().map(|t| t.token_type).collect()
        })
    }

    #[test]
    fn scans_keywords_and_punctuation() {
        let kinds = token_types("class Foo { }");
        assert_eq!(
            kinds,
            vec![
                TokenType::Class,
                TokenType::Identifier,
                TokenType::OpenBrace,
                TokenType::CloseBrace,
            ]
        );
    }

    #[test]
    fn scans_literals() {
        let kinds = token_types("var x = 42; var s = \"hi\"; var b = true;");
        assert!(kinds.contains(&TokenType::IntegerLiteral));
        assert!(kinds.contains(&TokenType::StringLiteral));
        assert!(kinds.contains(&TokenType::True));
        assert!(kinds.contains(&TokenType::Var));
        assert!(kinds.contains(&TokenType::Semicolon));
    }

    #[test]
    fn token_spans_the_source() {
        let ctx = CodeContext::new();
        let begin = ctx.with_current(|ctx| {
            let file = SourceFile::new(ctx, SourceFileType::Source, "test.vala", Some("class Foo"));
            ctx.add_source_file(&file);
            let token = Scanner::new(&file).read_token().expect("a token");
            assert_eq!(token.token_type, TokenType::Class);
            token.begin
        });
        assert_eq!(begin.line, 1);
        assert_eq!(begin.column, 1);
    }

    #[test]
    fn pops_doc_comments_while_scanning() {
        let ctx = CodeContext::new();
        let comments = ctx.with_current(|ctx| {
            let src = "/** the foo class */\nclass Foo { }";
            let file = SourceFile::new(ctx, SourceFileType::Source, "test.vala", Some(src));
            ctx.add_source_file(&file);
            let scanner = Scanner::new(&file);
            let mut comments = Vec::new();
            while let Some(token) = scanner.read_token() {
                while let Some(c) = scanner.pop_comment() {
                    comments.push((
                        token.token_type,
                        c.source_reference().map(|r| r.begin().line),
                    ));
                }
            }
            comments
        });
        // libvala attaches the doc comment to the `class` keyword that follows it.
        assert_eq!(comments, vec![(TokenType::Class, Some(1))]);
    }

    #[test]
    fn discards_ordinary_comments() {
        let ctx = CodeContext::new();
        let count = ctx.with_current(|ctx| {
            let src = "// line\n/* block */\nclass Foo { }";
            let file = SourceFile::new(ctx, SourceFileType::Source, "test.vala", Some(src));
            ctx.add_source_file(&file);
            let scanner = Scanner::new(&file);
            let mut count = 0;
            while scanner.read_token().is_some() {
                while scanner.pop_comment().is_some() {
                    count += 1;
                }
            }
            count
        });
        // libvala retains only `/** ... */` doc comments; these are dropped.
        assert_eq!(count, 0);
    }
}
