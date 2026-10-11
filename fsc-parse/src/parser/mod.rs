pub(crate) mod error;
use crate::ast::{
    BinOp, ConfigDecl, ConstDecl, Expr, ExprKind, FuncDecl, FuncDef, FunctionHeader, Item, Param,
    Stmt, Ty, UnaryOp,
};
use crate::ast::{NodeId, StmtKind};
use crate::lexer::token::{Token, TokenKind};
use crate::parser::error::{ParseError, ParseResult};
use fsc_diagnostics::Span;
pub struct TokenStream {
    tokens: Vec<Token>,
    cursor: usize,
    source: String,
}

impl TokenStream {
    pub(crate) fn new(tokens: Vec<Token>, source: String) -> Self {
        Self {
            tokens,
            cursor: 0,
            source,
        }
    }
    pub(crate) fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn current_offset(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map_or(self.source.len(), |token| token.span.start())
    }

    fn previous_token_end(&self) -> usize {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(self.current_offset(), |token| token.span.end())
    }

    fn span_consumed_from(&self, start: usize) -> Span {
        Span::new(start, self.previous_token_end().max(start))
    }

    fn error_span(&self) -> Span {
        if let Some(t) = self.tokens.get(self.cursor) {
            t.span
        } else {
            let end = self.source.len();
            Span::new(end, end)
        }
    }

    fn unexpected(&self, expected: &'static str) -> ParseError {
        match self.peek() {
            None => ParseError::UnexpectedEof {
                expected,
                span: self.error_span(),
            },
            Some(t) => ParseError::UnexpectedToken {
                got: t.description().into(),
                expected,
                span: self.error_span(),
            },
        }
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor).map(|t| &t.kind)
    }
    fn advance(&mut self) -> Option<&TokenKind> {
        let tok = self.tokens.get(self.cursor).map(|t| &t.kind);
        if !self.is_at_end() {
            self.cursor += 1;
        }
        tok
    }
    fn eat(&mut self, expected: &TokenKind) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &TokenKind, label: &'static str) -> ParseResult<()> {
        if self.eat(expected) {
            return Ok(());
        }
        Err(self.unexpected(label))
    }

    fn eat_ident(&mut self) -> Option<(String, Span)> {
        let span = self.tokens.get(self.cursor)?.span;
        if let Some(TokenKind::Ident(_)) = self.peek()
            && let Some(TokenKind::Ident(s)) = self.advance()
        {
            return Some((s.clone(), span));
        }
        None
    }
    fn expect_ident(&mut self) -> ParseResult<(String, Span)> {
        self.eat_ident()
            .ok_or_else(|| self.unexpected("identifier"))
    }
}

#[derive(Default)]
pub struct NodeIdGen {
    next: u32,
}

impl NodeIdGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn alloc(&mut self) -> NodeId {
        let id = NodeId(self.next);
        self.next += 1;
        id
    }
}

pub struct Parser {
    ts: TokenStream,
    ids: NodeIdGen,
}

struct DeclHead {
    ty: Ty,
    ty_span: Span,
    name: String,
    name_span: Span,
}

impl Parser {
    pub fn new(ts: TokenStream) -> Self {
        Self {
            ts,
            ids: NodeIdGen::new(),
        }
    }
    pub fn parse_item(&mut self) -> ParseResult<Item> {
        match self.ts.peek() {
            Some(TokenKind::KwExtern) => self.parse_function_declaration().map(Item::FuncDecl),
            Some(TokenKind::KwConst) => self.parse_const_declaration().map(Item::ConstDecl),
            Some(TokenKind::KwConfig) => self.parse_config_declaration().map(Item::ConfigDecl),
            Some(TokenKind::KwExport) => self.parse_exported_function().map(Item::FuncDef),
            Some(token) if token.is_type_keyword() => {
                self.parse_function_definition(false).map(Item::FuncDef)
            }
            _ => Err(self.ts.unexpected("top-level item")),
        }
    }
    pub fn is_at_end(&self) -> bool {
        self.ts.is_at_end()
    }

    fn parse_exported_function(&mut self) -> ParseResult<FuncDef> {
        self.ts.expect(&TokenKind::KwExport, "`export`")?;
        self.parse_function_definition(true)
    }

    fn parse_function_definition(&mut self, exported: bool) -> ParseResult<FuncDef> {
        let header = self.parse_function_header()?;
        let body = self.parse_block()?;
        Ok(FuncDef {
            id: self.ids.alloc(),
            header,
            body,
            exported,
        })
    }

    fn parse_function_declaration(&mut self) -> ParseResult<FuncDecl> {
        self.ts.expect(&TokenKind::KwExtern, "`extern`")?;
        let header = self.parse_function_header()?;
        self.ts.expect(&TokenKind::Semicolon, "`;`")?;

        Ok(FuncDecl {
            id: self.ids.alloc(),
            header,
        })
    }

    fn parse_const_declaration(&mut self) -> ParseResult<ConstDecl> {
        self.ts.expect(&TokenKind::KwConst, "`const`")?;
        let head = self.parse_decl_head()?;
        self.ts.expect(&TokenKind::Eq, "`=`")?;
        let initializer = self.parse_expr(0)?;
        self.ts.expect(&TokenKind::Semicolon, "`;`")?;

        Ok(ConstDecl {
            name: head.name,
            name_span: head.name_span,
            ty: head.ty,
            ty_span: head.ty_span,
            initializer,
        })
    }

    fn parse_config_declaration(&mut self) -> ParseResult<ConfigDecl> {
        self.ts.expect(&TokenKind::KwConfig, "`config`")?;
        let head = self.parse_decl_head()?;
        self.ts.expect(&TokenKind::Semicolon, "`;`")?;

        Ok(ConfigDecl {
            name: head.name,
            name_span: head.name_span,
            ty: head.ty,
            ty_span: head.ty_span,
        })
    }

    fn parse_function_header(&mut self) -> ParseResult<FunctionHeader> {
        let head = self.parse_decl_head()?;
        let params = self.parse_param_list()?;

        Ok(FunctionHeader {
            name: head.name,
            name_span: head.name_span,
            params,
            ret_ty: head.ty,
            ret_ty_span: head.ty_span,
        })
    }

    fn parse_decl_head(&mut self) -> ParseResult<DeclHead> {
        let (ty, ty_span) = self.parse_type_keyword()?;
        let (name, name_span) = self.ts.expect_ident()?;
        Ok(DeclHead {
            ty,
            ty_span,
            name,
            name_span,
        })
    }

    fn parse_type_keyword(&mut self) -> ParseResult<(Ty, Span)> {
        let start = self.ts.current_offset();
        let ty = match self.ts.peek() {
            Some(TokenKind::KwInt) => Ty::Int,
            Some(TokenKind::KwFloat) => Ty::Float,
            Some(TokenKind::KwVec3) => Ty::Vec3,
            Some(TokenKind::KwVoid) => Ty::Void,
            Some(TokenKind::KwBool) => Ty::Bool,
            Some(TokenKind::KwString) => Ty::Str,
            _ => return Err(self.ts.unexpected("type keyword")),
        };
        self.ts.advance();
        Ok((ty, self.ts.span_consumed_from(start)))
    }

    fn parse_param_list(&mut self) -> ParseResult<Vec<Param>> {
        self.ts.expect(&TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();

        if self.ts.eat(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            params.push(self.parse_param()?);
            if self.ts.eat(&TokenKind::RParen) {
                break;
            }
            self.ts.expect(&TokenKind::Comma, "`,`")?;
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> ParseResult<Param> {
        let (ty, ty_span) = self.parse_type_keyword()?;
        let (name, name_span) = self.ts.expect_ident()?;
        Ok(Param {
            name,
            name_span,
            ty,
            ty_span,
        })
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        let mut stmt = match self.ts.peek() {
            Some(TokenKind::KwIf) => self.parse_if(),
            Some(TokenKind::KwWhile) => self.parse_while(),
            _ => {
                let stmt = self.parse_stmt_inner()?;
                self.ts.expect(&TokenKind::Semicolon, "`;`")?;
                Ok(stmt)
            }
        }?;
        stmt.span = self.ts.span_consumed_from(start);
        Ok(stmt)
    }

    fn parse_stmt_inner(&mut self) -> ParseResult<Stmt> {
        match self.ts.peek() {
            Some(TokenKind::KwReturn) => self.parse_return(),
            Some(TokenKind::KwBreak) => {
                let start = self.ts.current_offset();
                self.ts.advance();
                Ok(Stmt::new(
                    self.ids.alloc(),
                    StmtKind::Break,
                    self.ts.span_consumed_from(start),
                ))
            }
            Some(TokenKind::Ident(_)) => self.parse_assign_or_expr_stmt(),
            Some(TokenKind::KwSysCall) => {
                let expr = self.parse_syscall()?;
                let span = expr.span;
                Ok(Stmt::new(self.ids.alloc(), StmtKind::ExprStmt(expr), span))
            }
            Some(TokenKind::KwPause) => self.parse_pause(),
            Some(token) if token.is_variable_type_keyword() => self.parse_var_decl(),
            _ => Err(self.ts.unexpected("statement")),
        }
    }

    fn parse_pause(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::KwPause, "`Pause`")?;
        self.ts.expect(&TokenKind::LParen, "`(`")?;
        let expr = self.parse_expr(0)?;
        self.ts.expect(&TokenKind::RParen, "`)`")?;
        Ok(Stmt::new(
            self.ids.alloc(),
            StmtKind::Pause(expr),
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_if(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::KwIf, "`if`")?;
        self.ts.expect(&TokenKind::LParen, "`(`")?;
        let cond = self.parse_expr(0)?;
        self.ts.expect(&TokenKind::RParen, "`)`")?;

        let then_body = self.parse_block()?;

        let else_body = if self.ts.eat(&TokenKind::KwElse) {
            if self.ts.peek() == Some(&TokenKind::KwIf) {
                Some(vec![self.parse_if()?])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Stmt::new(
            self.ids.alloc(),
            StmtKind::If {
                cond,
                then_body,
                else_body,
            },
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_while(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::KwWhile, "`while`")?;
        self.ts.expect(&TokenKind::LParen, "`(`")?;

        let cond = self.parse_expr(0)?;

        self.ts.expect(&TokenKind::RParen, "`)`")?;

        let body = self.parse_block()?;

        Ok(Stmt::new(
            self.ids.alloc(),
            StmtKind::While { cond, body },
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_block(&mut self) -> ParseResult<Vec<Stmt>> {
        self.ts.expect(&TokenKind::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        while !self.ts.eat(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        let head = self.parse_decl_head()?;
        let init = if self.ts.eat(&TokenKind::Eq) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };
        Ok(Stmt::new(
            self.ids.alloc(),
            StmtKind::VarDecl {
                name: head.name,
                name_span: head.name_span,
                ty: head.ty,
                ty_span: head.ty_span,
                init,
            },
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_assign_or_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        let (name, name_span) = self.ts.expect_ident()?;
        if self.ts.peek() == Some(&TokenKind::LParen) {
            let call = self.parse_call(name, name_span)?;
            let span = call.span;
            Ok(Stmt::new(self.ids.alloc(), StmtKind::ExprStmt(call), span))
        } else {
            let target = Expr::new(self.ids.alloc(), ExprKind::Var(name), name_span);
            let target = self.parse_member_suffix(target)?;
            self.ts.expect(&TokenKind::Eq, "`=`")?;
            let expr = self.parse_expr(0)?;
            Ok(Stmt::new(
                self.ids.alloc(),
                StmtKind::Assign { target, expr },
                self.ts.span_consumed_from(start),
            ))
        }
    }

    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::KwReturn, "`return`")?;

        if self.ts.peek() == Some(&TokenKind::Semicolon) {
            return Ok(Stmt::new(
                self.ids.alloc(),
                StmtKind::Return(None),
                self.ts.span_consumed_from(start),
            ));
        }

        let expr = self.parse_expr(0)?;
        Ok(Stmt::new(
            self.ids.alloc(),
            StmtKind::Return(Some(expr)),
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_expr(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut lhs = match self.ts.peek() {
            Some(TokenKind::IntLit(_)) => self.parse_int_literal(),
            Some(TokenKind::FloatLit(_)) => self.parse_float_literal(),
            Some(TokenKind::BoolLit(_)) => self.parse_bool_literal(),
            Some(TokenKind::StrLit(_)) => self.parse_string_literal(),
            Some(TokenKind::Ident(_)) => self.parse_identifier(),
            Some(TokenKind::LParen) => self.parse_group(),
            Some(TokenKind::Bang | TokenKind::Minus) => self.parse_unary(),
            Some(TokenKind::KwSysCall) => self.parse_syscall(),
            _ => Err(self.ts.unexpected("expression")),
        }?;

        lhs = self.parse_member_suffix(lhs)?;

        while let Some(tok) = self.ts.peek() {
            let Some((bp, op)) = binding_power(tok) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            self.ts.advance();
            let rhs = self.parse_expr(bp + 1)?;
            let span = lhs.span.cover(rhs.span);

            lhs = Expr::new(
                self.ids.alloc(),
                ExprKind::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }

        Ok(lhs)
    }

    fn parse_member_suffix(&mut self, mut base: Expr) -> ParseResult<Expr> {
        while self.ts.eat(&TokenKind::Dot) {
            let (member, member_span) = self.ts.expect_ident()?;
            let span = base.span.cover(member_span);
            base = Expr::new(
                self.ids.alloc(),
                ExprKind::Member {
                    base: Box::new(base),
                    member,
                    member_span,
                },
                span,
            );
        }
        Ok(base)
    }
    fn parse_syscall(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::KwSysCall, "`SysCall`")?;
        let args = self.parse_arg_list()?;
        Ok(Expr::new(
            self.ids.alloc(),
            ExprKind::SysCall { args },
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        let op = match self.ts.peek() {
            Some(TokenKind::Bang) => UnaryOp::Not,
            Some(TokenKind::Minus) => UnaryOp::Neg,
            _ => return Err(self.ts.unexpected("unary operator")),
        };
        self.ts.advance();
        let expr = self.parse_expr(6)?;

        Ok(Expr::new(
            self.ids.alloc(),
            ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
            self.ts.span_consumed_from(start),
        ))
    }

    fn parse_bool_literal(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        match self.ts.advance() {
            Some(TokenKind::BoolLit(b)) => Ok(Expr::new(
                self.ids.alloc(),
                ExprKind::BoolLit(*b),
                self.ts.span_consumed_from(start),
            )),
            _ => Err(self.ts.unexpected("boolean literal")),
        }
    }

    fn parse_int_literal(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        match self.ts.advance() {
            Some(TokenKind::IntLit(n)) => Ok(Expr::new(
                self.ids.alloc(),
                ExprKind::IntLit(*n),
                self.ts.span_consumed_from(start),
            )),
            _ => Err(self.ts.unexpected("`integer literal`")),
        }
    }
    fn parse_float_literal(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        match self.ts.advance() {
            Some(TokenKind::FloatLit(n)) => Ok(Expr::new(
                self.ids.alloc(),
                ExprKind::FloatLit(*n),
                self.ts.span_consumed_from(start),
            )),
            _ => Err(self.ts.unexpected("`float literal`")),
        }
    }
    fn parse_string_literal(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        match self.ts.advance() {
            Some(TokenKind::StrLit(n)) => Ok(Expr::new(
                self.ids.alloc(),
                ExprKind::StringLit(n.clone()),
                self.ts.span_consumed_from(start),
            )),
            _ => Err(self.ts.unexpected("`string literal`")),
        }
    }
    fn parse_group(&mut self) -> ParseResult<Expr> {
        let start = self.ts.current_offset();
        self.ts.expect(&TokenKind::LParen, "`(`")?;
        let mut expr = self.parse_expr(0)?;
        self.ts.expect(&TokenKind::RParen, "`)`")?;
        expr.span = self.ts.span_consumed_from(start);
        Ok(expr)
    }

    fn parse_identifier(&mut self) -> ParseResult<Expr> {
        let (name, name_span) = self.ts.expect_ident()?;
        match self.ts.peek() {
            Some(TokenKind::LParen) => self.parse_call(name, name_span),
            _ => Ok(Expr::new(self.ids.alloc(), ExprKind::Var(name), name_span)),
        }
    }

    fn parse_call(&mut self, name: String, name_span: Span) -> ParseResult<Expr> {
        let args = self.parse_arg_list()?;
        Ok(Expr::new(
            self.ids.alloc(),
            ExprKind::Call {
                callee: name,
                callee_span: name_span,
                args,
            },
            self.ts.span_consumed_from(name_span.start()),
        ))
    }
    fn parse_arg_list(&mut self) -> ParseResult<Vec<Expr>> {
        self.ts.expect(&TokenKind::LParen, "`(`")?;
        let mut args = Vec::new();

        if self.ts.eat(&TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expr(0)?);
            if self.ts.eat(&TokenKind::RParen) {
                break;
            }
            self.ts.expect(&TokenKind::Comma, "`,`")?;
        }
        Ok(args)
    }
}

fn binding_power(tok: &TokenKind) -> Option<(u8, BinOp)> {
    match tok {
        // logical
        TokenKind::PipePipe => Some((1, BinOp::Or)),
        TokenKind::AmpAmp => Some((2, BinOp::And)),
        // comparison
        TokenKind::EqEq => Some((3, BinOp::Eq)),
        TokenKind::BangEq => Some((3, BinOp::Neq)),
        TokenKind::Lt => Some((3, BinOp::Lt)),
        TokenKind::Gt => Some((3, BinOp::Gt)),
        TokenKind::LtEq => Some((3, BinOp::Le)),
        TokenKind::GtEq => Some((3, BinOp::Ge)),
        // arithmetic
        TokenKind::Plus => Some((4, BinOp::Add)),
        TokenKind::Minus => Some((4, BinOp::Sub)),
        TokenKind::Star => Some((5, BinOp::Mul)),
        TokenKind::Slash => Some((5, BinOp::Div)),
        _ => None,
    }
}

#[cfg(test)]
mod parse_expr_tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    use super::*;
    use crate::ast::{BinOp, Expr};
    use crate::lexer::tokenize;

    fn parse(src: &str) -> Expr {
        let lex_output = tokenize(src);
        assert!(
            lex_output.errors.is_empty(),
            "lex errors: {:?}",
            lex_output.errors
        );
        let mut parser = Parser::new(TokenStream::new(lex_output.tokens, src.to_string()));
        parser.parse_expr(0).expect("parse error")
    }

    #[test]
    fn comparison_lower_precedence_than_arithmetic() {
        // 1 + 2 == 3  →  (1 + 2) == 3
        let e = parse("1 + 2 == 3");

        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Eq,
                lhs: left,
                rhs: right,
            } => {
                assert!(matches!(right.kind, ExprKind::IntLit(3)));

                match left.kind {
                    ExprKind::BinOp {
                        op: BinOp::Add,
                        lhs: l,
                        rhs: r,
                    } => {
                        assert!(matches!(l.kind, ExprKind::IntLit(1)));
                        assert!(matches!(r.kind, ExprKind::IntLit(2)));
                    }
                    _ => panic!("left side should be addition"),
                }
            }
            _ => panic!("expected top-level equality"),
        }
    }

    #[test]
    fn logical_and_lower_than_comparison() {
        // a == b && c == d  →  (a == b) && (c == d)
        let e = parse("a == b && c == d");

        match e.kind {
            ExprKind::BinOp {
                op: BinOp::And,
                lhs: left,
                rhs: right,
            } => {
                // left: a == b
                match left.kind {
                    ExprKind::BinOp {
                        op: BinOp::Eq,
                        lhs: l,
                        rhs: r,
                    } => {
                        assert!(matches!(l.kind, ExprKind::Var(ref s) if s == "a"));
                        assert!(matches!(r.kind, ExprKind::Var(ref s) if s == "b"));
                    }
                    _ => panic!("left side should be equality"),
                }

                // right: c == d
                match right.kind {
                    ExprKind::BinOp {
                        op: BinOp::Eq,
                        lhs: l,
                        rhs: r,
                    } => {
                        assert!(matches!(l.kind, ExprKind::Var(ref s) if s == "c"));
                        assert!(matches!(r.kind, ExprKind::Var(ref s) if s == "d"));
                    }
                    _ => panic!("right side should be equality"),
                }
            }
            _ => panic!("expected top-level AND"),
        }
    }

    #[test]
    fn unary_not() {
        // !x
        let e = parse("!x");

        match e.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                expr,
            } => {
                assert!(matches!(expr.kind, ExprKind::Var(ref s) if s == "x"));
            }
            _ => panic!("expected unary NOT"),
        }
    }

    #[test]
    fn bool_literal_true() {
        assert_eq!(parse("true").kind, ExprKind::BoolLit(true));
    }

    #[test]
    fn bool_literal_false() {
        assert_eq!(parse("false").kind, ExprKind::BoolLit(false));
    }
    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let e = parse("1 + 2 * 3");

        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Add,
                lhs: left,
                rhs: right,
            } => {
                assert!(matches!(left.kind, ExprKind::IntLit(1)));
                match right.kind {
                    ExprKind::BinOp {
                        op: BinOp::Mul,
                        lhs: l,
                        rhs: r,
                    } => {
                        assert!(matches!(l.kind, ExprKind::IntLit(2)));
                        assert!(matches!(r.kind, ExprKind::IntLit(3)));
                    }
                    _ => panic!("left side should be addition"),
                }
            }
            _ => panic!("expected unary NOT"),
        }
    }

    #[test]
    fn variable_in_expression() {
        let e = parse("x + 1");
        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Add,
                lhs: left,
                rhs: right,
            } => {
                assert!(matches!(left.kind, ExprKind::Var(ref s) if s == "x"));
                assert!(matches!(right.kind, ExprKind::IntLit(1)));
            }
            _ => panic!("expected add"),
        }
    }

    #[test]
    fn chained_mixed_precedence() {
        let e = parse("1 + 2 * 3 + 4");

        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                match lhs.kind {
                    ExprKind::BinOp {
                        op: BinOp::Add,
                        lhs: l1,
                        rhs: r1,
                    } => {
                        assert!(matches!(l1.kind, ExprKind::IntLit(1)));

                        match r1.kind {
                            ExprKind::BinOp {
                                op: BinOp::Mul,
                                lhs: m1,
                                rhs: m2,
                            } => {
                                assert!(matches!(m1.kind, ExprKind::IntLit(2)));
                                assert!(matches!(m2.kind, ExprKind::IntLit(3)));
                            }
                            _ => panic!("expected multiplication in middle"),
                        }
                    }
                    _ => panic!("expected left nested addition"),
                }

                assert!(matches!(rhs.kind, ExprKind::IntLit(4)));
            }
            _ => panic!("expected top-level addition"),
        }
    }

    #[test]
    fn nested_groups() {
        let e = parse("((1 + 2)) * ((3 + 4))");

        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Mul,
                lhs,
                rhs,
            } => {
                match lhs.kind {
                    ExprKind::BinOp {
                        op: BinOp::Add,
                        lhs: l1,
                        rhs: r1,
                    } => {
                        assert!(matches!(l1.kind, ExprKind::IntLit(1)));
                        assert!(matches!(r1.kind, ExprKind::IntLit(2)));
                    }
                    _ => panic!("expected left side to be addition"),
                }

                match rhs.kind {
                    ExprKind::BinOp {
                        op: BinOp::Add,
                        lhs: l2,
                        rhs: r2,
                    } => {
                        assert!(matches!(l2.kind, ExprKind::IntLit(3)));
                        assert!(matches!(r2.kind, ExprKind::IntLit(4)));
                    }
                    _ => panic!("expected right side to be addition"),
                }
            }
            _ => panic!("expected top-level multiplication"),
        }
    }

    #[test]
    fn group_in_middle_of_chain() {
        let e = parse("1 * (2 + 3) * 4");
        match e.kind {
            ExprKind::BinOp {
                op: BinOp::Mul,
                lhs,
                rhs,
            } => {
                match lhs.kind {
                    ExprKind::BinOp {
                        op: BinOp::Mul,
                        lhs: l1,
                        rhs: r1,
                    } => {
                        assert!(matches!(l1.kind, ExprKind::IntLit(1)));

                        match r1.kind {
                            ExprKind::BinOp {
                                op: BinOp::Add,
                                lhs: a1,
                                rhs: a2,
                            } => {
                                assert!(matches!(a1.kind, ExprKind::IntLit(2)));
                                assert!(matches!(a2.kind, ExprKind::IntLit(3)));
                            }
                            _ => panic!("expected (2 + 3) in middle"),
                        }
                    }
                    _ => panic!("expected left multiplication"),
                }

                assert!(matches!(rhs.kind, ExprKind::IntLit(4)));
            }
            _ => panic!("expected top-level multiplication"),
        }
    }
}
