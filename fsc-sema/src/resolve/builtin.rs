use super::error::ResolveError;
use fsc_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Builtin {
    Syscall,
    Pause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuiltinContext {
    Value,
    Statement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuiltinArity {
    Exact(usize),
    AtLeast(usize),
}

impl Builtin {
    pub(super) fn resolve(name: &str) -> Option<Self> {
        match name {
            "syscall" => Some(Self::Syscall),
            "pause" => Some(Self::Pause),
            _ => None,
        }
    }

    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Syscall => "syscall",
            Self::Pause => "pause",
        }
    }

    pub(super) const fn supports_context(self, context: BuiltinContext) -> bool {
        match (self, context) {
            (Self::Syscall, BuiltinContext::Value | BuiltinContext::Statement)
            | (Self::Pause, BuiltinContext::Statement) => true,
            (Self::Pause, BuiltinContext::Value) => false,
        }
    }

    const fn arity(self) -> BuiltinArity {
        match self {
            Self::Syscall => BuiltinArity::AtLeast(2),
            Self::Pause => BuiltinArity::Exact(1),
        }
    }

    pub(super) fn validate_argument_count(
        self,
        actual: usize,
        invocation_span: Span,
    ) -> Result<(), ResolveError> {
        let arity = self.arity();
        let valid = match arity {
            BuiltinArity::Exact(expected) => actual == expected,
            BuiltinArity::AtLeast(minimum) => actual >= minimum,
        };
        if valid {
            return Ok(());
        }

        Err(ResolveError::BuiltinArgumentCount {
            name: self.name(),
            expected: arity,
            actual,
            invocation_span,
        })
    }
}
