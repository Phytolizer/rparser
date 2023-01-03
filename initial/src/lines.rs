use std::borrow::Cow;

use bstr::BStr;
use bstr::BString;
use bstr::ByteSlice;

use crate::line::CharInfo;
use crate::line::Line;
use crate::line::OwnedLine;

pub struct Lines<'a>(Vec<Line<'a>>);

impl<'a> Lines<'a> {
    pub fn new(input: &'a BStr) -> Self {
        Self(
            input
                .lines()
                .map(|line| Line::new(Cow::Borrowed(line.into())).build())
                .collect(),
        )
    }

    pub fn merge_escaped_newlines(mut self) -> Self {
        let mut builder = OwnedLine::empty();
        let mut write_idx = 0;
        for rd in 0..self.0.len() {
            let mut line = Line::empty();
            std::mem::swap(&mut self.0[rd], &mut line);
            if line.text.ends_with_str("\\") {
                builder.text.extend_from_slice(&line.text);
                builder.synthetic.extend_from_slice(&line.synthetic);
                builder.trivial.extend(
                    line.trivial
                        .iter()
                        .take(line.trivial.len() - 2)
                        .chain([true, true].iter()),
                );
            } else {
                if builder.text.is_empty() {
                    self.0[write_idx] = line;
                } else {
                    builder.text.extend_from_slice(&line.text);
                    builder.trivial.extend_from_slice(&line.trivial);
                    builder.synthetic.extend_from_slice(&line.synthetic);
                    self.0[write_idx] = builder.to_line();
                }
                write_idx += 1;
            }
        }
        if !builder.text.is_empty() {
            self.0[write_idx] = builder.to_line();
            write_idx += 1;
        }
        self.0.truncate(write_idx);
        self
    }

    pub fn delete_comments(mut self) -> Self {
        let mut builder = OwnedLine::empty();
        let mut comments = CommentState::new();
        let mut wr = 0;
        for rd in 0..self.0.len() {
            let mut line = Line::empty();
            std::mem::swap(&mut self.0[rd], &mut line);
            for info in line.chars() {
                builder.push(info);

                if let Some(Emit { ch, pop_count }) = should_emit(info.ch, &mut comments) {
                    backtrack(&mut builder, pop_count);
                    if ch != info.ch {
                        builder.push(CharInfo::new(ch, false, true));
                    }
                } else {
                    *builder.trivial.last_mut().unwrap() = true;
                }
                if !info.trivial {
                    comments.prev_char = info.ch;
                }
            }
            if let Some(Emit { ch, pop_count }) = should_emit(b'\n', &mut comments) {
                backtrack(&mut builder, pop_count);
                if ch != b'\n' {
                    builder.push(CharInfo::new(ch, false, true));
                }
            } else {
                *builder.trivial.last_mut().unwrap() = true;
            }
            comments.prev_char = b'\n';
            if !comments.in_block_comment {
                self.0[wr] = builder.to_line();
                wr += 1;
            }
        }
        self.0.truncate(wr);
        self
    }

    pub fn finish(self) -> BString {
        self.0
            .into_iter()
            .fold(vec![], |mut acc, line| {
                acc.extend(line.to_non_trivial());
                acc.push(b'\n');
                acc
            })
            .into()
    }
}

struct CommentState {
    in_string: bool,
    in_block_comment: bool,
    in_line_comment: bool,
    prev_char: u8,
}

struct Emit {
    ch: u8,
    pop_count: usize,
}

impl CommentState {
    fn new() -> Self {
        Self {
            in_string: false,
            in_block_comment: false,
            in_line_comment: false,
            prev_char: 0,
        }
    }
}

impl Emit {
    fn new(ch: u8) -> Self {
        Self { ch, pop_count: 0 }
    }
}

fn should_emit(ch: u8, comments: &mut CommentState) -> Option<Emit> {
    if comments.in_string {
        if ch == b'"' && comments.prev_char != b'\\' {
            comments.in_string = false;
        }
        Some(Emit::new(ch))
    } else if comments.in_block_comment && ch == b'/' && comments.prev_char == b'*' {
        comments.in_block_comment = false;
        None
    } else if comments.in_line_comment && ch == b'\n' {
        comments.in_line_comment = false;
        Some(Emit::new(ch))
    } else if comments.in_line_comment || comments.in_block_comment {
        None
    } else {
        match ch {
            b'/' => {
                if comments.prev_char == b'/' {
                    comments.in_line_comment = true;
                    Some(Emit {
                        ch: b' ',
                        pop_count: 2,
                    })
                } else {
                    Some(Emit::new(ch))
                }
            }
            b'*' => {
                if comments.prev_char == b'/' {
                    comments.in_block_comment = true;
                    Some(Emit {
                        ch: b' ',
                        pop_count: 2,
                    })
                } else {
                    Some(Emit::new(ch))
                }
            }
            b'"' => {
                comments.in_string = !comments.in_string;
                Some(Emit::new(ch))
            }
            _ => Some(Emit::new(ch)),
        }
    }
}

fn backtrack(builder: &mut OwnedLine, pop_count: usize) {
    let mut i = builder.text.len();
    for _ in 0..pop_count {
        while i > 0 && builder.trivial[i - 1] {
            i -= 1;
        }
        builder.trivial[i - 1] = true;
    }
}
