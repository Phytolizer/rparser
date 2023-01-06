use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::BuildHasherDefault;

use bstr::BString;
use bstr::ByteSlice;
use itertools::Itertools;
use itertools::MultiPeek;
use rand_core::RngCore;
use wyhash::WyHash;
use wyhash::WyRng;

use crate::token::Punct;
use crate::token::Token;

enum Directive {
    If,
    Ifdef,
    Ifndef,
    Elif,
    Else,
    Endif,
    Include,
    Define,
    Undef,
    Line,
    Error,
    Pragma,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing directive name")]
    MissingDirectiveName,
    #[error("invalid directive {0}")]
    InvalidDirective(BString),
    #[error("`elif` has no `if` to bind to")]
    MismatchedElif,
}

struct Hash(WyHash);
impl Default for Hash {
    fn default() -> Self {
        Self(WyHash::with_seed(WyRng::default().next_u64()))
    }
}

type MacroTable = HashMap<BString, BString, BuildHasherDefault<Hash>>;

struct Parser<'a, Tokens>
where
    Tokens: Iterator<Item = Token<'a>>,
{
    macros: MacroTable,
    tokens: MultiPeek<Tokens>,
    // one token may yield many.
    out_stack: VecDeque<Token<'a>>,
    directives: Vec<Directive>,
}

impl<'a, Tokens> Parser<'a, Tokens>
where
    Tokens: Iterator<Item = Token<'a>>,
{
    fn new(tokens: Tokens) -> Self {
        Self {
            macros: MacroTable::default(),
            tokens: tokens.multipeek(),
            out_stack: VecDeque::new(),
            directives: vec![],
        }
    }

    fn handle_iflike_directive(&mut self, directive: Directive) -> Result<(), ParseError> {
        match directive {
            Directive::If => {
                self.parse_condition()?;
            }
            Directive::Elif => {
                let top = self.directives.last().ok_or(ParseError::MismatchedElif)?;
                if !matches!(
                    top,
                    Directive::If | Directive::Ifdef | Directive::Ifndef | Directive::Elif
                ) {
                    return Err(ParseError::MismatchedElif);
                }
            }
            Directive::Ifdef | Directive::Ifndef => {}
            _ => unreachable!(),
        }
        Ok(())
    }
}

impl<'a, Tokens> Iterator for Parser<'a, Tokens>
where
    Tokens: Iterator<Item = Token<'a>>,
{
    type Item = Result<Token<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tok) = self.out_stack.pop_front() {
            return Some(Ok(tok));
        }

        match self.tokens.next()? {
            Token::Punct(Punct::Hash) => {
                // this next word should be one of the known directives
                match self.tokens.next() {
                    Some(Token::Ident(id)) => match id.as_bytes() {
                        b"if" => {
                            self.handle_iflike_directive(Directive::If)?;
                        }
                        b"ifdef" => {
                            self.handle_iflike_directive(Directive::Ifdef)?;
                        }
                        b"ifndef" => {
                            self.handle_iflike_directive(Directive::Ifndef)?;
                        }
                        b"elif" => {
                            self.handle_iflike_directive(Directive::Elif)?;
                        }
                        b"else" => {
                            self.handle_else()?;
                        }
                        b"endif" => {
                            self.handle_endif()?;
                        }
                        b"include" => {
                            self.handle_include()?;
                        }
                        b"define" => {
                            self.handle_define()?;
                        }
                        b"undef" => {
                            self.handle_undef()?;
                        }
                        b"line" => {
                            self.handle_line()?;
                        }
                        b"error" => {
                            self.handle_error()?;
                        }
                        b"pragma" => {
                            self.handle_pragma()?;
                        }
                        _ => return Some(Err(ParseError::InvalidDirective(id.to_owned()))),
                    },
                    Some(tok) => {
                        return Some(Err(ParseError::InvalidDirective(
                            format!("{tok}").into_bytes().into(),
                        )))
                    }
                    None => return Some(Err(ParseError::MissingDirectiveName)),
                }
            }
            result => {
                // eagerly consume the line
                loop {
                    match self.tokens.peek() {
                        Some(&Token::Eol | &Token::Eof) | None => break,
                        _ => {
                            self.out_stack.push_back(self.tokens.next().unwrap());
                        }
                    }
                }
                return Some(Ok(result));
            }
        }
        None
    }
}
