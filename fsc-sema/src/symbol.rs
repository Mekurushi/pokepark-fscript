use crate::types::Ty;
use fsc_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub type_span: Span,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamInfo {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub type_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConstValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String),
}

impl ConstValue {
    pub(crate) fn ty(&self) -> Ty {
        match self {
            Self::Int(_) => Ty::Int,
            Self::Float(_) => Ty::Float,
            Self::Bool(_) => Ty::Bool,
            Self::Str(_) => Ty::Str,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Param { index: u32 },
    Local,
    Const { value: ConstValue, value_span: Span },
    Config,
    Function { ret_ty: Ty, params: Vec<ParamInfo> },
}
#[derive(Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    pub fn insert(&mut self, symbol: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(symbol);
        id
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        let symbol = self.symbols.get(id.0 as usize);
        match symbol {
            Some(symbol) => symbol,
            None => todo!("Symbol not found"), //TODO: explicit error
        }
    }

    pub fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        let symbol = self.symbols.get_mut(id.0 as usize);
        match symbol {
            Some(symbol) => symbol,
            None => todo!("Symbol not found"), //TODO: explicit error
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (SymbolId, &Symbol)> {
        self.symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (SymbolId(index as u32), symbol))
    }
}
