use std::borrow::Cow;

use bstr::BStr;
use bstr::BString;
use itertools::izip;

pub(crate) struct Line<'a> {
    pub(crate) text: Cow<'a, BStr>,
    pub(crate) trivial: Vec<bool>,
    pub(crate) synthetic: Vec<bool>,
}

pub(crate) struct LineBuilder<'a> {
    text: Cow<'a, BStr>,
    trivial: Option<Vec<bool>>,
    synthetic: Option<Vec<bool>>,
}

pub(crate) struct OwnedLine {
    pub(crate) text: BString,
    pub(crate) trivial: Vec<bool>,
    pub(crate) synthetic: Vec<bool>,
}

#[derive(Clone, Copy)]
pub(crate) struct CharInfo {
    pub(crate) ch: u8,
    pub(crate) trivial: bool,
    pub(crate) synthetic: bool,
}

impl CharInfo {
    pub(crate) fn new(ch: u8, trivial: bool, synthetic: bool) -> Self {
        Self {
            ch,
            trivial,
            synthetic,
        }
    }
}

impl OwnedLine {
    pub(crate) fn empty() -> Self {
        Self {
            text: vec![].into(),
            trivial: vec![],
            synthetic: vec![],
        }
    }

    pub(crate) fn to_line(&mut self) -> Line<'static> {
        let mut temp = OwnedLine::empty();
        std::mem::swap(self, &mut temp);
        Line::new(Cow::Owned(temp.text))
            .with_synthetic(temp.synthetic)
            .with_trivial(temp.trivial)
            .build()
    }

    pub(crate) fn push(&mut self, info: CharInfo) {
        self.text.push(info.ch);
        self.trivial.push(info.trivial);
        self.synthetic.push(info.synthetic);
    }
}

impl<'a> Line<'a> {
    pub(crate) fn new(data: Cow<'a, BStr>) -> LineBuilder<'a> {
        LineBuilder {
            text: data,
            trivial: None,
            synthetic: None,
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            text: Cow::Borrowed(BStr::new(b"")),
            trivial: vec![],
            synthetic: vec![],
        }
    }

    pub(crate) fn to_non_trivial(&self) -> impl Iterator<Item = u8> + '_ {
        self.text
            .iter()
            .zip(self.trivial.iter())
            .filter_map(|(&ch, trivial)| (!trivial).then(|| ch))
    }

    pub(crate) fn chars(&self) -> impl Iterator<Item = CharInfo> + '_ {
        izip!(
            self.text.iter().copied(),
            self.trivial.iter().copied(),
            self.synthetic.iter().copied(),
        )
        .map(|(ch, trivial, synthetic)| CharInfo::new(ch, trivial, synthetic))
    }
}

impl<'a> LineBuilder<'a> {
    pub(crate) fn with_trivial(mut self, trivial: Vec<bool>) -> Self {
        self.trivial = Some(trivial);
        self
    }

    pub(crate) fn with_synthetic(mut self, synthetic: Vec<bool>) -> Self {
        self.synthetic = Some(synthetic);
        self
    }

    pub(crate) fn build(self) -> Line<'a> {
        Line {
            trivial: self.trivial.unwrap_or_else(|| vec![false; self.text.len()]),
            synthetic: self
                .synthetic
                .unwrap_or_else(|| vec![false; self.text.len()]),
            text: self.text,
        }
    }
}
