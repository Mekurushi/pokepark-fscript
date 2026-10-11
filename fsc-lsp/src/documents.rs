use crate::line_index::LineIndex;
use lsp_types::Position;
use lsp_types::{DidChangeTextDocumentParams, TextDocumentItem, Uri};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

pub(crate) struct Document {
    text: String,
    version: i32,
    line_index: LineIndex,
}

impl Document {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) const fn version(&self) -> i32 {
        self.version
    }

    pub(crate) fn position(&self, byte_offset: usize) -> Option<Position> {
        self.line_index.position(byte_offset)
    }
}

#[allow(clippy::mutable_key_type)]
pub(crate) struct Documents {
    entries: HashMap<Uri, Document>,
}

impl Documents {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn open(&mut self, item: TextDocumentItem) -> &Document {
        let document = Document {
            line_index: LineIndex::new(&item.text),
            text: item.text,
            version: item.version,
        };
        match self.entries.entry(item.uri) {
            Entry::Occupied(mut entry) => {
                entry.insert(document);
                entry.into_mut()
            }
            Entry::Vacant(entry) => entry.insert(document),
        }
    }

    pub(crate) fn change(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> Result<&Document, DocumentError> {
        let identifier = params.text_document;
        let document = self
            .entries
            .get_mut(&identifier.uri)
            .ok_or(DocumentError::DocumentNotOpen)?;

        if identifier.version <= document.version {
            return Err(DocumentError::StaleVersion {
                current: document.version,
                received: identifier.version,
            });
        }

        if params
            .content_changes
            .iter()
            .any(|change| change.range.is_some() || change.range_length.is_some())
        {
            return Err(DocumentError::UnexpectedIncrementalChange);
        }

        if let Some(change) = params.content_changes.last() {
            document.line_index = LineIndex::new(&change.text);
            document.text.clone_from(&change.text);
        }
        document.version = identifier.version;
        Ok(document)
    }

    pub(crate) fn close(&mut self, uri: &Uri) -> bool {
        self.entries.remove(uri).is_some()
    }
}

#[derive(Debug)]
pub(crate) enum DocumentError {
    DocumentNotOpen,
    StaleVersion { current: i32, received: i32 },
    UnexpectedIncrementalChange,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentNotOpen => formatter.write_str("document is not open"),
            Self::StaleVersion { current, received } => write!(
                formatter,
                "version {received} is not newer than current version {current}"
            ),
            Self::UnexpectedIncrementalChange => {
                formatter.write_str("received an incremental change while using full sync")
            }
        }
    }
}

impl std::error::Error for DocumentError {}
