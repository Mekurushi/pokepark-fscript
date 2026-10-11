use line_index::{LineIndex as RawLineIndex, TextSize, WideEncoding};
use lsp_types::Position;

pub(crate) struct LineIndex {
    inner: RawLineIndex,
}

impl LineIndex {
    pub(crate) fn new(text: &str) -> Self {
        Self {
            inner: RawLineIndex::new(text),
        }
    }

    pub(crate) fn position(&self, byte_offset: usize) -> Option<Position> {
        let offset = TextSize::try_from(byte_offset).ok()?;
        let line_col = self.inner.try_line_col(offset)?;
        // UTF-16 is the mandatory encoding to stay backwards compatible
        let wide = self.inner.to_wide(WideEncoding::Utf16, line_col)?;
        Some(Position::new(wide.line, wide.col))
    }
}
