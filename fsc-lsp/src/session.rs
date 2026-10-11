use crate::documents::Documents;
use lsp_types::InitializeParams;

pub(crate) struct Session {
    capabilities: SessionCapabilities,
    documents: Documents,
}

impl Session {
    pub(crate) fn new(initialize_params: &InitializeParams) -> Self {
        Self {
            capabilities: SessionCapabilities::from_initialize(initialize_params),
            documents: Documents::new(),
        }
    }

    pub(crate) const fn capabilities(&self) -> SessionCapabilities {
        self.capabilities
    }

    pub(crate) fn documents_mut(&mut self) -> &mut Documents {
        &mut self.documents
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SessionCapabilities {
    related_information: bool,
}

impl SessionCapabilities {
    fn from_initialize(params: &InitializeParams) -> Self {
        let related_information = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|capabilities| capabilities.publish_diagnostics.as_ref())
            .and_then(|capabilities| capabilities.related_information)
            .unwrap_or(false);

        Self {
            related_information,
        }
    }

    pub(crate) const fn related_information(self) -> bool {
        self.related_information
    }
}
