use clippy_utils::diagnostics::span_lint;
use clippy_utils::sym;
use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::def::Res;
use rustc_hir::{Body, ExprKind, HirId, Mutability, PatKind, QPath, StmtKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;

// use crate::methods::REDUNDANT_IDEMPOTENT_CALLS_INFO;

use super::REDUNDANT_IDEMPOTENT_CALLS;
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    // If the body of the funciton is a block
    if let ExprKind::Block(block, _) = &body.value.kind {
        // tracks which local variables have already had an idempotent method applied
        // HirId : name of the method that was applied
        let mut map = FxIndexMap::default();
        walk_block(cx, block, &mut map);
    }
}

fn walk_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    for stmt in block.stmts {
        match &stmt.kind {
            StmtKind::Let(local) => check_let(cx, local, map),
            StmtKind::Semi(expr) | StmtKind::Expr(expr) => {
                check_expr(cx, expr, map);
            },
            _ => {},
        }
    }

    if let Some(expr) = block.expr {
        return check_expr(cx, expr, map);
    }
    None
}

fn invalidate_left_value<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>, map: &mut FxIndexMap<HirId, Symbol>) {
    if let Some(hir_id) = path_to_local(expr) {
        map.shift_remove(&hir_id);
    }
}

fn check_let<'tcx>(cx: &LateContext<'tcx>, local: &'tcx rustc_hir::LetStmt<'tcx>, map: &mut FxIndexMap<HirId, Symbol>) {
    if let Some(init) = local.init {
        let mut expr = init;
        // inrelevant and inserted by the compiler
        while let ExprKind::DropTemps(inner) = &expr.kind {
            expr = inner;
        }

        if let ExprKind::MethodCall(method, receiver, args, _) = &expr.kind
            && is_idempotent(method.ident.name)
        {
            check_method_call(cx, expr, method, receiver, args, map);
            // record the new binding if it is a simple identifier
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                map.insert(hir_id, method.ident.name);
            }
        } else {
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                // try to inherit the alias symbol otherwise we check the expr
                if !try_inherit_alias(expr, hir_id, map) {
                    if let Some(symbol) = check_expr(cx, expr, map) {
                        map.insert(hir_id, symbol);
                    }
                }
            } else {
                check_expr(cx, expr, map);
            }
        }
    }
}

fn is_idempotent(name: Symbol) -> bool {
    matches!(
        name,
        sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs | sym::floor
    )
}

fn path_to_local(expr: &rustc_hir::Expr<'_>) -> Option<HirId> {
    if let ExprKind::Path(QPath::Resolved(None, path)) = &expr.kind
        && let Res::Local(hir_id) = path.res
    {
        Some(hir_id)
    } else {
        None
    }
}

// returns true if there is an insertion into the map
fn check_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    match &expr.kind {
        ExprKind::MethodCall(method, receiver, args, _) => check_method_call(cx, expr, method, receiver, args, map),
        ExprKind::Assign(left_value, right_value, _) => check_assign(cx, left_value, right_value, map),
        ExprKind::AssignOp(_, left_value, right_value) => check_assign(cx, left_value, right_value, map),
        ExprKind::Loop(block, _, _, _) => check_loop(cx, block, map),
        ExprKind::Match(_, arms, _) => check_match(cx, arms, map),
        ExprKind::Call(_, args) => check_func_args(cx, args, map),
        ExprKind::Closure(closure) => check_closure(cx, closure, map),
        ExprKind::Block(block, _) => walk_block(cx, block, map),
        ExprKind::Ret(Some(expr)) => check_expr(cx, expr, map),
        ExprKind::DropTemps(inner) => check_expr(cx, inner, map),
        ExprKind::If(cond, then_block, else_block) => {
            check_expr(cx, cond, map);
            check_if(cx, then_block, else_block, map)
        },
        ExprKind::Binary(_, left, right) => {
            check_expr(cx, left, map);
            check_expr(cx, right, map);
            None
        },
        ExprKind::Tup(exprs) => {
            for expr in *exprs {
                check_expr(cx, expr, map);
            }
            None
        },
        ExprKind::Struct(_, fields, base) => {
            for field in *fields {
                check_expr(cx, field.expr, map);
            }
            if let rustc_hir::StructTailExpr::Base(base_expr) = base {
                check_expr(cx, base_expr, map);
            }
            None
        },
        ExprKind::Array(exprs) => {
            for expr in *exprs {
                check_expr(cx, expr, map);
            }
            None
        },
        _ => None,
    }
}
fn check_func_args<'tcx>(cx: &LateContext<'tcx>, args: &'tcx [rustc_hir::Expr<'tcx>], map: &mut FxIndexMap<HirId, Symbol>) -> Option<Symbol> {
    for arg in args {
        if let ExprKind::AddrOf(_, Mutability::Mut, inner) = &arg.kind
            && let Some(hir_id) = path_to_local(inner)
        {
            map.shift_remove(&hir_id);
        } else {
            check_expr(cx, arg, map);
        }
    }
    return None;
}
fn check_method_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    method: &rustc_hir::PathSegment<'tcx>,
    receiver: &'tcx rustc_hir::Expr<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    // invalidate args mutable
    check_func_args(cx, args, map);

    // if the receiver is borrowed with a "&mut self"
    if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
        let fn_sig = cx.tcx.fn_sig(def_id).skip_binder();
        if let Some(first_ty) = fn_sig.inputs().skip_binder().get(0) {
            // to get the self
            if let rustc_middle::ty::Ref(_, _, Mutability::Mut) = first_ty.kind() {
                invalidate_left_value(receiver, map);
            }
        }
    }

    if is_idempotent(method.ident.name)
        && let Some(hir_id) = path_to_local(receiver)
        && let Some(recorded_method) = map.get(&hir_id)
    {
        if *recorded_method == method.ident.name {
            span_lint(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
            );
        } else {
            map.insert(hir_id, method.ident.name);
            return Some(method.ident.name);
        }
    } else if is_idempotent(method.ident.name)
        && let ExprKind::MethodCall(recursive_method, ..) = receiver.kind
    {
        //println!("the og name is {:#?}, the upwards is
        // {:#?}",method.ident.name,recursive_method.ident.name);
        if method.ident.name == recursive_method.ident.name {
            span_lint(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
            );
        }
    } else if is_idempotent(method.ident.name) {
        return Some(method.ident.name);
    }
    None
}

fn check_if<'tcx>(
    cx: &LateContext<'tcx>,
    then_block: &'tcx rustc_hir::Expr<'tcx>,
    else_block: &Option<&'tcx rustc_hir::Expr<'tcx>>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    // each branch gets its own clone to avoid cross contamination
    let mut then_map = map.clone();
    if let ExprKind::Block(block, _) = &then_block.kind {
        walk_block(cx, block, &mut then_map);
    }

    if let Some(else_expr) = else_block {
        if let ExprKind::Block(block, _) = &else_expr.kind {
            let mut else_map = map.clone();
            walk_block(cx, block, &mut else_map);
            // conservative merge
            map.retain(|hir_id, method| then_map.get(hir_id) == Some(method) && else_map.get(hir_id) == Some(method));
        } else {
            //"else if" or other node diff than block,
            // clear everything that the then branch might have changed
            map.retain(|hir_id, method| then_map.get(hir_id) == Some(method));
            check_expr(cx, else_expr, map);
        }
    } else {
        // no "else" branch
        map.retain(|hir_id, method| then_map.get(hir_id) == Some(method));
    }
    None
}

fn check_assign<'tcx>(
    cx: &LateContext<'tcx>,
    left_value: &'tcx rustc_hir::Expr<'tcx>,
    right_value: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {

    if let Some(dst_hir_id) = path_to_local(left_value) {
        if !try_inherit_alias(right_value, dst_hir_id, map) {
            if let Some(symbol) = check_expr(cx, right_value, map) {
                map.insert(dst_hir_id, symbol);
            } else {
                invalidate_left_value(left_value, map);
            }
        }
    } else {
            check_expr(cx, right_value, map);
    }
    None
}

fn check_loop<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    let mut loop_map = map.clone();
    walk_block(cx, block, &mut loop_map);

    // A state survives only if it existed before the loop and if it was left completely identical
    // inside the loop body.
    map.retain(|hir_id, method| loop_map.get(hir_id) == Some(method));
    None
}

fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    arms: &'tcx [rustc_hir::Arm<'tcx>],
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    if arms.is_empty() {
        return None;
    }

    let mut arm_maps = Vec::new();
    for arm in arms {
        let mut arm_map = map.clone();

        if let Some(guard) = arm.guard {
            check_expr(cx, guard, &mut arm_map);
        }

        check_expr(cx, arm.body, &mut arm_map);
        arm_maps.push(arm_map);
    }
    // A variable keeps its tracking state if it is present and identical across all match arms.
    map.retain(|hir_id, method| arm_maps.iter().all(|arm_map| arm_map.get(hir_id) == Some(method)));

    None
}

fn check_closure<'tcx>(
    cx: &LateContext<'tcx>,
    closure: &'tcx rustc_hir::Closure<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> Option<Symbol> {
    // invalidate any variables captured mutably
    for capture in cx.typeck_results().closure_min_captures_flattened(closure.def_id) {
        if capture.mutability == Mutability::Mut {
            map.shift_remove(&capture.get_root_variable());
        }
    }
    None
}

fn try_inherit_alias(
    expr: &rustc_hir::Expr<'_>,
    pat_hir_id: HirId,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
    if let Some(src_hir_id) = path_to_local(expr)
        && let Some(&symbol) = map.get(&src_hir_id)
    {
        map.insert(pat_hir_id, symbol);
        true
    } else {
        false
    }
}

