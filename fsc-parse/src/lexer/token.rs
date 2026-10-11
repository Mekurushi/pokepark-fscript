use crate::lexer::error::LexerError;
use fsc_diagnostics::Span;
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\f]+")] // whitespace
#[logos(skip r"\n")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))] // line comments TODO: check greedy
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")] // block comments
pub enum TokenKind {
    // --- Type Keywords ---
    #[token("int", priority = 10)]
    KwInt,

    #[token("float", priority = 10)]
    KwFloat,

    #[token("vec3", priority = 10)]
    KwVec3,

    #[token("void", priority = 10)]
    KwVoid,

    #[token("bool", priority = 10)]
    KwBool,

    #[token("string", priority = 10)]
    KwString,

    // --- Control Flow ---
    #[token("if", priority = 10)]
    KwIf,

    #[token("else", priority = 10)]
    KwElse,

    #[token("while", priority = 10)]
    KwWhile,

    #[token("return", priority = 10)]
    KwReturn,

    #[token("break", priority = 10)]
    KwBreak,

    // --- Declaration keywords ---
    #[token("export", priority = 10)]
    KwExport,

    #[token("extern", priority = 10)]
    KwExtern,

    #[token("const", priority = 10)]
    KwConst,

    #[token("config", priority = 10)]
    KwConfig,

    // --- misc ---
    #[token("SysCall", priority = 10)]
    KwSysCall,

    #[token("Pause", priority = 10)]
    KwPause,

    // --- boolean literals ---
    #[token("true",  priority = 10, callback = |_| true)]
    #[token("false", priority = 10, callback = |_| false)]
    BoolLit(bool),

    #[regex(r#""[^"]*""#, |lex| {
    let s = lex.slice();
    s[1..s.len()-1].to_string() // strip quotes
})]
    StrLit(String),

    // --- Numeric literals ---
    #[regex(
        r"(?:[0-9]+\.[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+",
        |lex| lex.slice().parse::<f32>().ok(),
        priority = 4
    )]
    FloatLit(f32),

    // --- Integer literals ---
    /// Hexadecimal integer; higher priority so it's not splitting
    #[regex(r"0[xX][0-9a-fA-F]+", |lex| {
        i32::from_str_radix(&lex.slice()[2..], 16).ok()
    }, priority = 5)]
    /// pure decimal
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i32>().ok(), priority = 2)]
    IntLit(i32),

    // --- Identifier ---
    /// non-keyword identifier e.g. function names
    /// Must start with a letter or underscore
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),

    // --- Arithmetic ---
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // --- Comparison operators ---
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,

    // --- Logical operators ---
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("!")]
    Bang,

    // --- Assignment ---
    #[token("=")]
    Eq,

    // --- Punctuation ---
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,

    #[token(".")]
    Dot,

    // --- Delimiters ---
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
}

impl TokenKind {
    pub const fn is_type_keyword(&self) -> bool {
        matches!(
            self,
            Self::KwInt
                | Self::KwFloat
                | Self::KwVec3
                | Self::KwVoid
                | Self::KwBool
                | Self::KwString
        )
    }

    pub const fn is_variable_type_keyword(&self) -> bool {
        self.is_type_keyword() && !matches!(self, Self::KwVoid)
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::KwInt => "`int`",
            Self::KwFloat => "`float`",
            Self::KwVec3 => "`vec3`",
            Self::KwVoid => "`void`",
            Self::KwBool => "`boolean`",
            Self::KwString => "`string`",
            Self::KwIf => "`if`",
            Self::KwElse => "`else`",
            Self::KwWhile => "`while`",
            Self::KwReturn => "`return`",
            Self::KwBreak => "`break`",
            Self::KwExport => "`export`",
            Self::KwExtern => "`extern`",
            Self::KwConst => "`const`",
            Self::KwConfig => "`config`",
            Self::KwSysCall => "`SysCall`",
            Self::KwPause => "`Pause`",
            Self::BoolLit(_) => "`boolean literal`",
            Self::IntLit(_) => "`integer literal`",
            Self::FloatLit(_) => "`float literal`",
            Self::StrLit(_) => "`string literal`",
            Self::Ident(_) => "`identifier`",
            Self::Plus => "`+`",
            Self::Minus => "`-`",
            Self::Star => "`*`",
            Self::Slash => "`/`",
            Self::Percent => "`%`",
            Self::EqEq => "`==`",
            Self::BangEq => "`!=`",
            Self::LtEq => "`<=`",
            Self::GtEq => "`>=`",
            Self::Lt => "`<`",
            Self::Gt => "`>`",
            Self::AmpAmp => "`&&`",
            Self::PipePipe => "`||`",
            Self::Bang => "`!`",
            Self::Eq => "`=`",
            Self::Comma => "`,`",
            Self::Semicolon => "`;`",
            Self::Dot => "`.`",
            Self::LParen => "`(`",
            Self::RParen => "`)`",
            Self::LBrace => "`{`",
            Self::RBrace => "`}`",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexerError>,
}
