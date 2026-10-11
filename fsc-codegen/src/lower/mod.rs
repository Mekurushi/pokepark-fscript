mod label_ctx;

use crate::error::{CodegenError, CodegenResult};
use crate::frame::{FrameLayout, plan_frame};
use crate::lower::label_ctx::LabelCtx;
use fsb_assembler::Assembler;
use fsc_sema::hir::{BinOp, Expr, FuncDef, Stmt, SysCallArg, UnaryOp};
use fsc_sema::types::Ty;

struct FunctionCx<'hir, 'asm> {
    function: &'hir FuncDef,
    frame: FrameLayout,
    labels: LabelCtx,
    asm: &'asm mut Assembler,
}

pub fn lower_func(func: &FuncDef, asm: &mut Assembler) -> CodegenResult<()> {
    let frame = plan_frame(func)?;
    let mut cx = FunctionCx {
        function: func,
        frame,
        labels: LabelCtx::new(),
        asm,
    };

    cx.asm
        .define_function(&cx.function.name, cx.function.exported)
        .map_err(Into::<CodegenError>::into)?;

    if cx.frame.local_slot_count() > 0 {
        cx.asm.emit_grow_stack(cx.frame.local_slot_count());
    }

    for stmt in &func.body {
        lower_stmt(stmt, &mut cx)?;
    }
    cx.asm.end_function()?;
    Ok(())
}

fn lower_stmt(stmt: &Stmt, cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    match stmt {
        Stmt::Return(expr) => lower_return(expr, cx),
        Stmt::Break => match cx.labels.loop_end() {
            Some(label) => Ok(cx.asm.emit_jmp(label)?),
            None => todo!("error?"),
        },
        Stmt::ReturnVoid => {
            cx.asm.emit_ret(cx.frame.frame_size());
            Ok(())
        }
        Stmt::Assign {
            target,
            value: expr,
        } => {
            lower_expr(expr, cx)?;
            let slot = cx.frame.resolve(target)?;
            cx.asm.emit_store_arg(slot.0);
            Ok(())
        }
        Stmt::VarDecl { local, init } => {
            if let Some(expr) = init {
                lower_expr(expr, cx)?;
                let slot = cx.frame.resolve(&fsc_sema::hir::Place::new(*local))?;
                cx.asm.emit_store_arg(slot.0);
            }
            Ok(())
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            lower_expr(cond, cx)?;
            match else_body {
                None => {
                    let end = cx.labels.fresh_label("if_end");
                    cx.asm.emit_jz(&end)?;
                    for s in then_body {
                        lower_stmt(s, cx)?;
                    }
                    cx.asm.define_label(&end)?;
                }
                Some(else_stmts) => {
                    let else_lbl = cx.labels.fresh_label("else");
                    let end = cx.labels.fresh_label("if_end");
                    cx.asm.emit_jz(&else_lbl)?;
                    for s in then_body {
                        lower_stmt(s, cx)?;
                    }
                    cx.asm.emit_jmp(&end)?;
                    cx.asm.define_label(&else_lbl)?;
                    for s in else_stmts {
                        lower_stmt(s, cx)?;
                    }
                    cx.asm.define_label(&end)?;
                }
            }
            Ok(())
        }
        Stmt::ExprStmt(expr) => {
            match expr {
                Expr::Call { callee, args, .. } => {
                    lower_call(callee, args, cx)?;
                }
                Expr::SysCall {
                    args,
                    subtype,
                    page,
                    func,
                    ..
                } => lower_syscall(args, cx, *subtype, *page, *func)?,
                _ => lower_expr(expr, cx)?,
            }
            Ok(())
        }
        Stmt::While { cond, body } => {
            lower_while(cond, body, cx)?;
            Ok(())
        }
        Stmt::Pause(expr) => {
            lower_expr(expr, cx)?;

            cx.asm.emit_delay_load();
            Ok(())
        }
    }
}

fn lower_while(cond: &Expr, body: &[Stmt], cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    let labels = cx.labels.enter_loop();

    cx.asm.define_label(&labels.header)?;

    lower_expr(cond, cx)?;
    cx.asm.emit_jz(&labels.end)?;

    for s in body {
        lower_stmt(s, cx)?;
    }

    cx.asm.emit_jmp(&labels.header)?;
    cx.asm.define_label(&labels.end)?;

    cx.labels.exit_loop();
    Ok(())
}
fn lower_return(expr: &Expr, cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    lower_expr(expr, cx)?;
    cx.asm.emit_retv(cx.frame.frame_size());
    Ok(())
}

fn lower_expr(expr: &Expr, cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    match expr {
        Expr::IntLit { value, .. } => {
            emit_int_lit(*value, cx.asm);
        }
        Expr::FloatLit { value, .. } => {
            cx.asm.emit_push_imm(value.to_bits());
        }
        Expr::BoolLit { value, .. } => {
            if *value {
                cx.asm.emit_push(1);
            } else {
                cx.asm.emit_push(0);
            }
        }
        Expr::StrLit { value, .. } => {
            cx.asm.emit_lstr(value)?;
        }

        Expr::Load { place, ty: _ty } => {
            let slot = cx.frame.resolve(place)?;
            cx.asm.emit_load_arg(slot.0);
        }

        Expr::Unary {
            op,
            expr: expression,
            ty,
        } => {
            // TODO: optimize push literal directly
            lower_expr(expression, cx)?;
            emit_unary(op, ty, cx.asm);
        }
        Expr::BinOp { op, lhs, rhs, .. } if matches!(op, BinOp::And | BinOp::Or) => match op {
            BinOp::And => emit_and_short_circuit(lhs, rhs, cx)?,
            BinOp::Or => emit_or_short_circuit(lhs, rhs, cx)?,
            _ => unreachable!(),
        },
        Expr::BinOp {
            op,
            lhs,
            rhs,
            ty: _,
        } => {
            //TODO: for Eq == 0 optimization to eq0
            lower_expr(lhs, cx)?;
            lower_expr(rhs, cx)?;

            emit_binop(op, lhs.ty(), cx.asm);
            // TODO: original scripts are saving in arg and load again; check if this is really
            // everytime necessary
        }
        Expr::Call { callee, args, .. } => {
            lower_call(callee, args, cx)?;
            cx.asm.emit_push_result();
        }
        Expr::SysCall {
            page,
            func,
            subtype,
            args,
            ..
        } => {
            lower_syscall(args, cx, *subtype, *page, *func)?;
            cx.asm.emit_push_result();
        }
    }
    Ok(())
}

fn lower_syscall(
    args: &[SysCallArg],
    cx: &mut FunctionCx<'_, '_>,
    subtype: u8,
    page: u8,
    func: u16,
) -> CodegenResult<()> {
    for arg in args.iter().rev() {
        match arg {
            SysCallArg::Value(expr) => lower_expr(expr, cx)?,
            SysCallArg::StackRef(place) => {
                let slot = cx.frame.resolve(place)?;
                cx.asm.emit_load_arg_ref();
                cx.asm.emit_load_arg(slot.0);
            }
        }
    }
    cx.asm.emit_syscall(subtype, page, func);
    Ok(())
}

fn lower_call(callee: &str, args: &[Expr], cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    // TODO: rev() used because of calling convention; check how to make this more explicit
    for arg in args.iter().rev() {
        lower_expr(arg, cx)?;
    }
    cx.asm.emit_call(callee)?;
    Ok(())
}
fn emit_and_short_circuit(
    lhs: &Expr,
    rhs: &Expr,
    cx: &mut FunctionCx<'_, '_>,
) -> CodegenResult<()> {
    let false_label = cx.labels.fresh_label("and_false");
    let end_label = cx.labels.fresh_label("and_end");

    lower_expr(lhs, cx)?;
    cx.asm.emit_jz(&false_label)?;

    lower_expr(rhs, cx)?;
    cx.asm.emit_jz(&false_label)?;

    cx.asm.emit_push(1);
    cx.asm.emit_jmp(&end_label)?;

    cx.asm.define_label(&false_label)?;
    cx.asm.emit_push(0);

    cx.asm.define_label(&end_label)?;
    Ok(())
}

fn emit_or_short_circuit(lhs: &Expr, rhs: &Expr, cx: &mut FunctionCx<'_, '_>) -> CodegenResult<()> {
    let true_label = cx.labels.fresh_label("or_true");
    let end_label = cx.labels.fresh_label("or_end");

    lower_expr(lhs, cx)?;
    cx.asm.emit_jnz(&true_label)?;

    lower_expr(rhs, cx)?;
    cx.asm.emit_jnz(&true_label)?;

    cx.asm.emit_push(0);
    cx.asm.emit_jmp(&end_label)?;

    cx.asm.define_label(&true_label)?;
    cx.asm.emit_push(1);

    cx.asm.define_label(&end_label)?;
    Ok(())
}
fn emit_int_lit(value: i32, asm: &mut Assembler) {
    match i16::try_from(value) {
        Ok(small) => asm.emit_push(small),
        Err(_) => asm.emit_push_imm(value.cast_unsigned()),
        //TODO: check whether push_imm makes
        // sense here
    }
}

fn emit_binop(op: &BinOp, operand_ty: &Ty, asm: &mut Assembler) {
    // TODO: refactor unreachable out
    match op {
        BinOp::Add => match operand_ty {
            Ty::Int => asm.emit_add(),
            Ty::Float => asm.emit_fadd(),
            Ty::Bool | Ty::Vec3 => unreachable!("add on unsupported type"),
            Ty::Void => unreachable!("add on void"),
            Ty::Str => unreachable!("add on str"),
        },
        BinOp::Sub => match operand_ty {
            Ty::Int => asm.emit_sub(),
            Ty::Float => asm.emit_fsub(),
            Ty::Bool | Ty::Vec3 => unreachable!("sub on unsupported type"),
            Ty::Void => unreachable!("sub on void"),
            Ty::Str => unreachable!("sub on str"),
        },
        BinOp::Mul => match operand_ty {
            Ty::Int => asm.emit_mul(),
            Ty::Float => asm.emit_fmul(),
            Ty::Bool | Ty::Vec3 => unreachable!("mul on unsupported type"),
            Ty::Void => unreachable!("mul on void"),
            Ty::Str => unreachable!("mul on str"),
        },
        BinOp::Div => match operand_ty {
            Ty::Int => asm.emit_div(),
            Ty::Float => asm.emit_fdiv(),
            Ty::Bool | Ty::Vec3 => unreachable!("div on unsupported type"),
            Ty::Void => unreachable!("div on void"),
            Ty::Str => unreachable!("div on str"),
        },

        BinOp::Eq => match operand_ty {
            Ty::Float => asm.emit_feq(),
            _ => asm.emit_eq(),
        },
        BinOp::Neq => match operand_ty {
            Ty::Float => asm.emit_fneq(),
            _ => asm.emit_neq(),
        },
        BinOp::Gt => match operand_ty {
            Ty::Float => asm.emit_fgt(),
            _ => asm.emit_gt(),
        },
        BinOp::Ge => match operand_ty {
            Ty::Float => asm.emit_fge(),
            _ => asm.emit_ge(),
        },
        BinOp::Lt => match operand_ty {
            Ty::Float => asm.emit_flt(),
            _ => asm.emit_lt(),
        },
        BinOp::Le => match operand_ty {
            Ty::Float => asm.emit_fle(),
            _ => asm.emit_le(),
        },

        BinOp::And | BinOp::Or => unreachable!("should be in short circuit logic"),
    }
}

fn emit_unary(op: &UnaryOp, operand_ty: &Ty, asm: &mut Assembler) {
    match (op, operand_ty) {
        (UnaryOp::Not, Ty::Bool) => asm.emit_eq0(),
        (UnaryOp::Neg, Ty::Int) => asm.emit_neg(),
        (UnaryOp::Neg, Ty::Float) => asm.emit_fneg(),
        _ => unreachable!("invalid unary operation reached code generation"),
    }
}
