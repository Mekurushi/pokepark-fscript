mod render;
mod render_json;

pub use render::render_diagnostics;
pub use render_json::render_diagnostics_json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Parse,
    Semantic,
    Codegen,
    Assembly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    #[must_use]
    pub fn range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }

    #[must_use]
    pub const fn cover(self, other: Self) -> Self {
        Self::new(
            if self.start < other.start {
                self.start
            } else {
                other.start
            },
            if self.end > other.end {
                self.end
            } else {
                other.end
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    span: Span,
    message: String,
    style: LabelStyle,
}

impl Label {
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self::new(span, message, LabelStyle::Primary)
    }

    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self::new(span, message, LabelStyle::Secondary)
    }

    fn new(span: Span, message: impl Into<String>, style: LabelStyle) -> Self {
        Self {
            span,
            message: message.into(),
            style,
        }
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn style(&self) -> LabelStyle {
        self.style
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    stage: Stage,
    severity: Severity,
    message: String,
    labels: Vec<Label>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(stage: Stage, message: impl Into<String>) -> Self {
        Self::new(stage, Severity::Error, message)
    }

    #[must_use]
    pub fn warning(stage: Stage, message: impl Into<String>) -> Self {
        Self::new(stage, Severity::Warning, message)
    }

    fn new(stage: Stage, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            stage,
            severity,
            message: message.into(),
            labels: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    #[must_use]
    pub const fn stage(&self) -> Stage {
        self.stage
    }

    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }
}
