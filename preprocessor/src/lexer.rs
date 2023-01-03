use bstr::BStr;

use crate::token::Punct;
use crate::token::Token;

struct Lexer<'a> {
    input: &'a BStr,
    pos: usize,
    at_line_start: bool,
    in_directive: bool,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a BStr) -> Self {
        Self {
            input,
            pos: 0,
            at_line_start: true,
            in_directive: false,
        }
    }

    fn get(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn move_on(&mut self) {
        let c = self.get();
        self.pos += 1;
        if let Some(b'\n') = c {
            self.at_line_start = true;
            self.in_directive = false;
        }
    }

    fn end_token(&mut self, k: Token<'a>) -> Token<'a> {
        if self.at_line_start && k.is_hash() {
            self.in_directive = true;
        }

        self.at_line_start = false;
        k
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos + 1).copied()
    }

    fn skip_whitespace(&mut self) -> Option<Token<'a>> {
        loop {
            match self.get() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.move_on();
                }
                Some(b'\n') => {
                    self.move_on();
                    return Some(Token::Eol);
                }
                _ => return None,
            }
        }
    }

    fn scan_ident(&mut self) -> Token<'a> {
        let start = self.pos;
        loop {
            match self.get() {
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => {
                    self.move_on();
                }
                _ => break,
            }
        }
        let end = self.pos;
        self.end_token(Token::Ident(&self.input[start..end]))
    }

    fn scan_number(&mut self) -> Option<Token<'a>> {
        let first = self.get().unwrap();
        let start = self.pos;
        self.move_on();
        if first == b'.' {
            match self.get() {
                Some(b'0'..=b'9') => {}
                _ => {
                    self.pos = start;
                    return None;
                }
            }
        }

        loop {
            match self.get() {
                Some(
                    b'0'..=b'9'
                    | b'.'
                    | b'a'..=b'd'
                    // skip 'e'
                    | b'f'..=b'o'
                    // skip 'p'
                    | b'q'..=b'z'
                    | b'A'..=b'D'
                    // skip 'E'
                    | b'F'..=b'O'
                    // skip 'P'
                    | b'Q'..=b'Z'
                    | b'_',
                ) => {
                    self.move_on();
                }
                Some(b'e' | b'E' | b'p' | b'P') => {
                    self.move_on();
                    if let Some(b'+' | b'-') = self.get() {
                        self.move_on();
                    }
                }
                _ => break,
            }
        }
        let end = self.pos;
        Some(self.end_token(Token::Number(&self.input[start..end])))
    }

    fn scan_string_lit(&mut self) -> Option<Token<'a>> {
        let first = self.get().unwrap();
        let terminator = match first {
            b'"' => b'"',
            b'\'' => b'\'',
            b'<' => b'>',
            _ => unreachable!("string literal starts with wrong char"),
        };
        let start = self.pos;
        self.move_on();
        if let Some(b':' | b'%') = self.get() {
            self.pos = start;
            return None;
        }

        loop {
            match self.get() {
                Some(ch) if ch == terminator => {
                    self.move_on();
                    let end = self.pos;
                    return Some(self.end_token(Token::StringLit(&self.input[start..end])));
                }
                Some(b'\\') if first != b'<' => {
                    self.move_on();
                    self.move_on();
                }
                Some(b'\n') => break,
                _ => {
                    self.move_on();
                }
            }
        }
        if first == b'<' {
            self.pos = start;
            return None;
        }
        let end = self.pos;
        Some(self.end_token(Token::Other(&self.input[start..end])))
    }

    fn scan_punct(&mut self) -> Token<'a> {
        let first = self.get().unwrap();
        self.move_on();

        // check digraphs
        match first {
            b'<' => match self.get() {
                Some(b':') => {
                    self.move_on();
                    return self.end_token(Token::Punct(Punct::LBrack));
                }
                Some(b'%') => {
                    self.move_on();
                    return self.end_token(Token::Punct(Punct::LBrace));
                }
                _ => {}
            },
            b'%' => match self.get() {
                Some(b'>') => {
                    self.move_on();
                    return self.end_token(Token::Punct(Punct::RBrace));
                }
                Some(b':') => {
                    self.move_on();
                    match self.get() {
                        Some(b'%') if self.peek() == Some(b':') => {
                            self.move_on();
                            self.move_on();
                            return self.end_token(Token::Punct(Punct::HashHash));
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            b':' => match self.get() {
                Some(b'>') => {
                    self.move_on();
                    return self.end_token(Token::Punct(Punct::RBrack));
                }
                _ => {}
            },
            _ => {}
        }

        // not a digraph
        match first {
            b'[' => self.end_token(Token::Punct(Punct::LBrack)),
            b']' => self.end_token(Token::Punct(Punct::RBrack)),
            b'(' => self.end_token(Token::Punct(Punct::LParen)),
            b')' => self.end_token(Token::Punct(Punct::RParen)),
            b'.' => {
                if self.get() == Some(b'.') && self.peek() == Some(b'.') {
                    self.move_on();
                    self.move_on();
                    self.end_token(Token::Punct(Punct::Ellipsis))
                } else {
                    self.end_token(Token::Punct(Punct::Period))
                }
            }
            b'-' => match self.get() {
                Some(b'>') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::Arrow))
                }
                Some(b'-') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::MinusMinus))
                }
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::MinusEq))
                }
                _ => self.end_token(Token::Punct(Punct::Minus)),
            },
            b'+' => match self.get() {
                Some(b'+') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::PlusPlus))
                }
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::PlusEq))
                }
                _ => self.end_token(Token::Punct(Punct::Plus)),
            },
            b'&' => match self.get() {
                Some(b'&') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::AmpAmp))
                }
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::AmpEq))
                }
                _ => self.end_token(Token::Punct(Punct::Amp)),
            },
            b'*' => {
                if let Some(b'=') = self.get() {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::StarEq))
                } else {
                    self.end_token(Token::Punct(Punct::Star))
                }
            }
            b'~' => self.end_token(Token::Punct(Punct::Tilde)),
            b'!' => {
                if let Some(b'=') = self.get() {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::BangEq))
                } else {
                    self.end_token(Token::Punct(Punct::Bang))
                }
            }
            b'/' => {
                if let Some(b'=') = self.get() {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::SlashEq))
                } else {
                    self.end_token(Token::Punct(Punct::Slash))
                }
            }
            b'%' => {
                if let Some(b'=') = self.get() {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::PercentEq))
                } else {
                    self.end_token(Token::Punct(Punct::Percent))
                }
            }
            b'<' => match self.get() {
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::LtEq))
                }
                Some(b'<') => {
                    self.move_on();
                    if self.get() == Some(b'=') {
                        self.move_on();
                        self.end_token(Token::Punct(Punct::LtLtEq))
                    } else {
                        self.end_token(Token::Punct(Punct::LtLt))
                    }
                }
                _ => self.end_token(Token::Punct(Punct::Lt)),
            },
            b'>' => match self.get() {
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::GtEq))
                }
                Some(b'>') => {
                    self.move_on();
                    if self.get() == Some(b'=') {
                        self.move_on();
                        self.end_token(Token::Punct(Punct::GtGtEq))
                    } else {
                        self.end_token(Token::Punct(Punct::GtGt))
                    }
                }
                _ => self.end_token(Token::Punct(Punct::Gt)),
            },
            b'=' => {
                if self.get() == Some(b'=') {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::EqEq))
                } else {
                    self.end_token(Token::Punct(Punct::Eq))
                }
            }
            b'^' => {
                if self.get() == Some(b'=') {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::CaretEq))
                } else {
                    self.end_token(Token::Punct(Punct::Caret))
                }
            }
            b'|' => match self.get() {
                Some(b'|') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::PipePipe))
                }
                Some(b'=') => {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::PipeEq))
                }
                _ => self.end_token(Token::Punct(Punct::Pipe)),
            },
            b'?' => self.end_token(Token::Punct(Punct::Question)),
            b':' => self.end_token(Token::Punct(Punct::Colon)),
            b',' => self.end_token(Token::Punct(Punct::Comma)),
            b'#' => {
                if self.get() == Some(b'#') {
                    self.move_on();
                    self.end_token(Token::Punct(Punct::HashHash))
                } else {
                    self.end_token(Token::Punct(Punct::Hash))
                }
            }
            b'{' => self.end_token(Token::Punct(Punct::LBrace)),
            b'}' => self.end_token(Token::Punct(Punct::RBrace)),
            b';' => self.end_token(Token::Punct(Punct::Semicolon)),
            _ => unreachable!("impossible punctuation"),
        }
    }

    fn scan_other(&mut self) -> Token<'a> {
        let start = self.pos;
        self.move_on();
        self.end_token(Token::Other(&self.input[start..self.pos]))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(t) = self.skip_whitespace() {
            return Some(t);
        }

        match self.get() {
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => Some(self.scan_ident()),
            Some(b'0'..=b'9' | b'.') => {
                Some(self.scan_number().unwrap_or_else(|| self.scan_punct()))
            }
            Some(b'"' | b'\'' | b'<') => {
                let result = if self.in_directive || self.get() != Some(b'<') {
                    self.scan_string_lit()
                } else {
                    None
                };
                Some(result.unwrap_or_else(|| self.scan_punct()))
            }
            Some(
                b'!'
                | b'#'
                | b'%'..=b'&'
                | b'('..=b'-'
                | b'/'
                | b':'..=b';'
                | b'='..=b'?'
                | b'['..=b'^'
                | b'{'..=b'~',
            ) => Some(self.scan_punct()),
            Some(_) => Some(self.scan_other()),
            _ => None,
        }
    }
}

pub fn lex<'a>(input: &'a BStr) -> impl Iterator<Item = Token<'a>> {
    Lexer::new(input).chain(std::iter::once(Token::Eof))
}
