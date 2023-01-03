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
    curr_directive: Option<Directive>,
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
            curr_directive: None,
        }
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
                            self.curr_directive = Some(Directive::If);
                        }
                        b"ifdef" => {
                            self.curr_directive = Some(Directive::Ifdef);
                        }
                        b"ifndef" => {
                            self.curr_directive = Some(Directive::Ifndef);
                        }
                        b"elif" => {
                            self.curr_directive = Some(Directive::Elif);
                        }
                        b"endif" => {
                            self.curr_directive = Some(Directive::Endif);
                        }
                        b"include" => {
                            self.curr_directive = Some(Directive::Include);
                        }
                        b"define" => {
                            self.curr_directive = Some(Directive::Define);
                        }
                        b"undef" => {
                            self.curr_directive = Some(Directive::Undef);
                        }
                        b"line" => {
                            self.curr_directive = Some(Directive::Line);
                        }
                        b"error" => {
                            self.curr_directive = Some(Directive::Error);
                        }
                        b"pragma" => {
                            self.curr_directive = Some(Directive::Pragma);
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
