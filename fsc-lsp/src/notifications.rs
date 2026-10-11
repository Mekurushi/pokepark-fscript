use crate::diagnostics::check_document;
use crate::documents::DocumentError;
use crate::session::Session;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    PublishDiagnosticsParams,
};

pub(crate) fn handle_notification(
    notification: lsp_server::Notification,
    session: &mut Session,
) -> Result<Option<PublishDiagnosticsParams>, NotificationError> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = serde_json::from_value::<DidOpenTextDocumentParams>(notification.params)?;
            let uri = params.text_document.uri.clone();
            let related_information = session.capabilities().related_information();
            let document = session.documents_mut().open(params.text_document);
            log::debug!(
                "opened {uri:?} at version {} ({} bytes)",
                document.version(),
                document.text().len(),
            );
            Ok(Some(check_document(uri, document, related_information)))
        }
        DidChangeTextDocument::METHOD => {
            let params =
                serde_json::from_value::<DidChangeTextDocumentParams>(notification.params)?;
            let uri = params.text_document.uri.clone();
            let related_information = session.capabilities().related_information();
            let document = session.documents_mut().change(params)?;
            log::debug!(
                "updated document to version {} ({} bytes)",
                document.version(),
                document.text().len(),
            );
            Ok(Some(check_document(uri, document, related_information)))
        }
        DidCloseTextDocument::METHOD => {
            let params = serde_json::from_value::<DidCloseTextDocumentParams>(notification.params)?;
            let uri = params.text_document.uri;
            if session.documents_mut().close(&uri) {
                log::debug!("closed {uri:?}");
            }
            Ok(Some(PublishDiagnosticsParams::new(uri, Vec::new(), None)))
        }
        _ => Ok(None),
    }
}

#[derive(Debug)]
pub(crate) enum NotificationError {
    InvalidParams(serde_json::Error),
    Document(DocumentError),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParams(error) => write!(formatter, "invalid parameters: {error}"),
            Self::Document(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for NotificationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidParams(error) => Some(error),
            Self::Document(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for NotificationError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidParams(error)
    }
}

impl From<DocumentError> for NotificationError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}
