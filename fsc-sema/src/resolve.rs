use crate::error::{SemaError, SemaResult};
use crate::symbol::{ConstValue, ParamInfo, Symbol, SymbolId, SymbolKind, SymbolTable};
use crate::types::Ty;
use fsc_diagnostics::Span;
use fsc_parse::ast;
use fsc_parse::ast::NodeId;
use std::collections::HashMap;

pub struct Resolutions {
    references: HashMap<NodeId, SymbolId>,
    frame_symbols: Vec<SymbolId>,
}

pub(crate) struct ResolvedFunction {
    pub(crate) symbol: SymbolId,
    pub(crate) resolutions: Resolutions,
}

impl Resolutions {
    fn new() -> Self {
        Self {
            references: HashMap::new(),
            frame_symbols: Vec::new(),
        }
    }

    fn insert(&mut self, node: NodeId, sym: SymbolId) {
        self.references.insert(node, sym);
    }
    pub fn symbol(&self, node: NodeId) -> SymbolId {
        let sym_id = self.references.get(&node);
        match sym_id {
            Some(sym_id) => *sym_id,
            None => todo!("Symbol not found"), //TODO: explicit error
        }
    }

    pub(crate) fn frame_symbols(&self) -> &[SymbolId] {
        &self.frame_symbols
    }

    fn add_frame_symbol(&mut self, symbol: SymbolId) {
        self.frame_symbols.push(symbol);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    File,
    Function,
    Block,
}
#[derive(Debug)]
struct Scope {
    kind: ScopeKind,
    map: HashMap<String, ScopeEntry>,
}
#[derive(Debug, Clone, Copy)]
struct ScopeEntry {
    symbol: SymbolId,
    declaration_span: Span,
}
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

// TODO: check assertions; currently more development check than state enforcements
impl ScopeStack {
    pub(crate) fn new() -> Self {
        Self { scopes: vec![] }
    }
    fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope {
            kind,
            map: HashMap::new(),
        });
    }
    fn current_kind(&self) -> Option<ScopeKind> {
        self.scopes.last().map(|s| s.kind)
    }

    fn enter_file_scope(&mut self) {
        assert!(
            self.scopes.is_empty(),
            "file scope already \
        initialized"
        );

        self.push_scope(ScopeKind::File);
    }

    fn enter_function_scope(&mut self) {
        assert_eq!(
            self.current_kind(),
            Some(ScopeKind::File),
            "function scope must be entered from file scope"
        );
        self.push_scope(ScopeKind::Function);
    }

    fn enter_block_scope(&mut self) {
        assert!(
            matches!(
                self.current_kind(),
                Some(ScopeKind::Function | ScopeKind::Block)
            ),
            "block scope must be inside function or block"
        );
        self.push_scope(ScopeKind::Block);
    }

    fn exit_scope(&mut self) {
        assert!(!self.scopes.is_empty(), "cannot exit empty scope stack");
        assert_ne!(
            self.current_kind(),
            Some(ScopeKind::File),
            "cannot exit root scope"
        );

        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, sym_id: SymbolId, name_span: Span) -> SemaResult<()> {
        let current = self.scopes.last_mut();
        let Some(scope) = current else {
            unreachable!("unreachable initialized with at least one scope")
        };
        if let Some(original) = scope.map.get(name) {
            return Err(SemaError::DuplicateDeclaration {
                name: name.to_string(),
                duplicate_span: name_span,
                original_span: original.declaration_span,
            });
        }
        scope.map.insert(
            name.to_string(),
            ScopeEntry {
                symbol: sym_id,
                declaration_span: name_span,
            },
        );
        Ok(())
    }

    fn lookup(&self, name: &str, reference_span: Span) -> SemaResult<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.map.get(name) {
                return Ok(entry.symbol);
            }
        }
        Err(SemaError::UndeclaredName {
            name: name.to_string(),
            reference_span,
        })
    }
}

pub fn declare_items(
    script: &ast::Script,
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
) -> SemaResult<()> {
    scope.enter_file_scope();
    for item in &script.items {
        match item {
            ast::Item::FuncDef(func) => {
                declare_function(&func.header, scope, symbols)?;
            }
            ast::Item::FuncDecl(declaration) => {
                validate_unique_params(&declaration.header.params)?;
                declare_function(&declaration.header, scope, symbols)?;
            }
            ast::Item::ConstDecl(constant) => declare_constant(constant, scope, symbols)?,
            ast::Item::ConfigDecl(config) => declare_config(config, scope, symbols)?,
        }
    }
    Ok(())
}

fn declare_config(
    config: &ast::ConfigDecl,
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
) -> SemaResult<()> {
    let symbol = symbols.insert(Symbol {
        name: config.name.clone(),
        name_span: config.name_span,
        ty: Ty::from(&config.ty),
        type_span: config.ty_span,
        kind: SymbolKind::Config,
    });
    scope.declare(&config.name, symbol, config.name_span)
}

fn declare_function(
    header: &ast::FunctionHeader,
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
) -> SemaResult<()> {
    let params = header
        .params
        .iter()
        .map(|param| ParamInfo {
            name: param.name.clone(),
            name_span: param.name_span,
            ty: Ty::from(&param.ty),
            type_span: param.ty_span,
        })
        .collect();
    let symbol = symbols.insert(Symbol {
        name: header.name.clone(),
        name_span: header.name_span,
        ty: Ty::from(&header.ret_ty),
        type_span: header.ret_ty_span,
        kind: SymbolKind::Function {
            ret_ty: Ty::from(&header.ret_ty),
            params,
        },
    });
    scope.declare(&header.name, symbol, header.name_span)
}

fn declare_constant(
    constant: &ast::ConstDecl,
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
) -> SemaResult<()> {
    let value = match &constant.initializer.kind {
        ast::ExprKind::IntLit(value) => ConstValue::Int(*value),
        ast::ExprKind::FloatLit(value) => ConstValue::Float(*value),
        ast::ExprKind::BoolLit(value) => ConstValue::Bool(*value),
        ast::ExprKind::StringLit(value) => ConstValue::Str(value.clone()),
        _ => {
            return Err(SemaError::ConstantInitializerMustBeLiteral {
                initializer_span: constant.initializer.span,
            });
        }
    };
    let symbol = symbols.insert(Symbol {
        name: constant.name.clone(),
        name_span: constant.name_span,
        ty: Ty::from(&constant.ty),
        type_span: constant.ty_span,
        kind: SymbolKind::Const {
            value,
            value_span: constant.initializer.span,
        },
    });
    scope.declare(&constant.name, symbol, constant.name_span)
}

fn validate_unique_params(params: &[ast::Param]) -> SemaResult<()> {
    let mut declarations = HashMap::new();
    for param in params {
        if let Some(original_span) = declarations.insert(&param.name, param.name_span) {
            return Err(SemaError::DuplicateDeclaration {
                name: param.name.clone(),
                duplicate_span: param.name_span,
                original_span,
            });
        }
    }
    Ok(())
}

pub fn resolve_params(
    params: &[ParamInfo],
    scope: &mut ScopeStack,
    symbol_table: &mut SymbolTable,
) -> SemaResult<Vec<SymbolId>> {
    let mut symbols = Vec::with_capacity(params.len());
    for (index, param) in params.iter().enumerate() {
        let sym_id = symbol_table.insert(Symbol {
            name: param.name.clone(),
            name_span: param.name_span,
            ty: param.ty.clone(),
            type_span: param.type_span,
            kind: SymbolKind::Param {
                index: index as u32,
            },
        });
        scope.declare(&param.name, sym_id, param.name_span)?;
        symbols.push(sym_id);
    }
    Ok(symbols)
}

pub fn resolve_fn(
    func: &ast::FuncDef,
    symbols: &mut SymbolTable,
    scope: &mut ScopeStack,
) -> SemaResult<ResolvedFunction> {
    scope.enter_function_scope();
    let fn_symbol_id = scope.lookup(&func.header.name, func.header.name_span)?;
    let mut resolutions = Resolutions::new();
    let (params, ..) = match symbols.get(fn_symbol_id).kind.clone() {
        SymbolKind::Function { params, ret_ty } => (params, ret_ty),
        _ => todo!(),
    };
    for parameter in resolve_params(&params, scope, symbols)? {
        resolutions.add_frame_symbol(parameter);
    }

    resolve_stmts(&func.body, scope, symbols, &mut resolutions)?;
    scope.exit_scope();
    Ok(ResolvedFunction {
        symbol: fn_symbol_id,
        resolutions,
    })
}

fn resolve_stmts(
    stmts: &[ast::Stmt],
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
    resolutions: &mut Resolutions,
) -> SemaResult<()> {
    for stmt in stmts {
        resolve_stmt(stmt, scope, symbols, resolutions)?;
    }
    Ok(())
}

fn resolve_stmt(
    stmt: &ast::Stmt,
    scope: &mut ScopeStack,
    symbols: &mut SymbolTable,
    resolutions: &mut Resolutions,
) -> SemaResult<()> {
    match &stmt.kind {
        ast::StmtKind::Return(expr) => {
            if let Some(e) = expr {
                resolve_expr(e, scope, resolutions)?;
            }
            Ok(())
        }

        ast::StmtKind::VarDecl {
            name,
            name_span,
            ty,
            ty_span,
            init,
        } => {
            if let Some(e) = init {
                resolve_expr(e, scope, resolutions)?;
            }
            let sym_id = symbols.insert(Symbol {
                name: name.clone(),
                name_span: *name_span,
                ty: Ty::from(ty),
                type_span: *ty_span,
                kind: SymbolKind::Local,
            });
            scope.declare(name, sym_id, *name_span)?;
            resolutions.insert(stmt.id, sym_id);
            resolutions.add_frame_symbol(sym_id);
            Ok(())
        }

        ast::StmtKind::Assign { target, expr } => {
            if !matches!(
                target.kind,
                ast::ExprKind::Var(_) | ast::ExprKind::Member { .. }
            ) {
                return Err(SemaError::InvalidAssignmentTarget {
                    target_span: target.span,
                });
            }
            resolve_expr(target, scope, resolutions)?;
            resolve_expr(expr, scope, resolutions)?;
            Ok(())
        }

        ast::StmtKind::If {
            cond,
            then_body,
            else_body,
        } => {
            resolve_expr(cond, scope, resolutions)?;

            scope.enter_block_scope();
            resolve_stmts(then_body, scope, symbols, resolutions)?;
            scope.exit_scope();

            if let Some(else_stmts) = else_body {
                scope.enter_block_scope();
                resolve_stmts(else_stmts, scope, symbols, resolutions)?;
                scope.exit_scope();
            }
            Ok(())
        }
        ast::StmtKind::While { cond, body } => {
            resolve_expr(cond, scope, resolutions)?;

            scope.enter_block_scope();
            resolve_stmts(body, scope, symbols, resolutions)?;
            scope.exit_scope();

            Ok(())
        }
        ast::StmtKind::ExprStmt(expr) => {
            resolve_expr(expr, scope, resolutions)?;
            Ok(())
        }

        ast::StmtKind::Pause(expr) => {
            resolve_expr(expr, scope, resolutions)?;
            Ok(())
        }
        ast::StmtKind::Break => Ok(()),
    }
}

fn resolve_expr(
    expr: &ast::Expr,
    scope: &ScopeStack,
    resolutions: &mut Resolutions,
) -> SemaResult<()> {
    match &expr.kind {
        ast::ExprKind::IntLit(_)
        | ast::ExprKind::FloatLit(_)
        | ast::ExprKind::BoolLit(_)
        | ast::ExprKind::StringLit(_) => Ok(()),

        ast::ExprKind::Var(name) => {
            let sym_id = scope.lookup(name, expr.span)?;
            resolutions.insert(expr.id, sym_id);
            Ok(())
        }

        ast::ExprKind::Member { base, .. } => resolve_expr(base, scope, resolutions),

        ast::ExprKind::BinOp { lhs, rhs, .. } => {
            resolve_expr(lhs, scope, resolutions)?;
            resolve_expr(rhs, scope, resolutions)
        }

        ast::ExprKind::Unary { expr: inner, .. } => resolve_expr(inner, scope, resolutions),

        ast::ExprKind::Call {
            callee,
            callee_span,
            args,
        } => {
            let sym_id = scope.lookup(callee, *callee_span)?;
            resolutions.insert(expr.id, sym_id);
            for arg in args {
                resolve_expr(arg, scope, resolutions)?;
            }
            Ok(())
        }
        ast::ExprKind::SysCall { args } => {
            for arg in &args[2..] {
                resolve_expr(arg, scope, resolutions)?;
            }
            Ok(())
        }
    }
}
