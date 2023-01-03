use std::fmt::Display;

use bstr::BStr;
use convert_case::Case;
use convert_case::Casing;

pub enum Token<'a> {
    Ident(&'a BStr),
    StringLit(&'a BStr),
    Number(&'a BStr),
    Punct(Punct),
    Other(&'a BStr),
    Eol,
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub enum Punct {
    Period,
    Arrow,
    PlusPlus,
    MinusMinus,
    Amp,
    Plus,
    Minus,
    Tilde,
    Bang,
    Slash,
    Percent,
    LtLt,
    GtGt,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    BangEq,
    Caret,
    Pipe,
    AmpAmp,
    PipePipe,
    Question,
    StarEq,
    SlashEq,
    PercentEq,
    PlusEq,
    MinusEq,
    LtLtEq,
    GtGtEq,
    AmpEq,
    CaretEq,
    PipeEq,
    HashHash,

    LBrack,
    RBrack,
    LParen,
    RParen,
    Star,
    Comma,
    Colon,
    Eq,
    Hash,

    LBrace,
    RBrace,
    Semicolon,
    Ellipsis,
}

impl<'a> Token<'a> {
    pub(crate) fn is_hash(&self) -> bool {
        return matches!(self, Token::Punct(Punct::Hash));
    }
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(v) => write!(f, "{{ident '{v}'}}"),
            Self::StringLit(v) => write!(f, "{{string_lit '{v}'}}"),
            Self::Number(v) => write!(f, "{{number '{v}'}}"),
            Self::Punct(p) => {
                let p = format!("{p:?}")
                    .from_case(Case::Pascal)
                    .to_case(Case::Snake);
                write!(f, "{{punct .{p}}}")
            }
            Self::Other(v) => write!(f, "{{other '{v}'}}"),
            Self::Eol => write!(f, "{{EOL}}"),
            Self::Eof => write!(f, "{{EOF}}"),
        }
    }
}
