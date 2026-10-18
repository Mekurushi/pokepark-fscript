mod check;
mod error;
mod results;

pub(crate) use check::check;
pub(crate) use results::{RawSyscallResults, ResolvedSelectors};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorKind {
    Page,
    Function,
}

impl SelectorKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Function => "function",
        }
    }

    const fn maximum(self) -> u16 {
        match self {
            Self::Page => u8::MAX as u16,
            Self::Function => u16::MAX,
        }
    }
}
