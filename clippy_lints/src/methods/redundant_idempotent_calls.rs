use clippy_utils::consts::ConstEvalCtxt;
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
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
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

fn invalidate_left_value<'tcx>(
    expr: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &[rustc_hir::Expr<'_>])>,
) {
    if let Some(hir_id) = path_to_local(expr) {
        map.shift_remove(&hir_id);
    }
}

fn check_let<'tcx>(
    cx: &LateContext<'tcx>,
    local: &'tcx rustc_hir::LetStmt<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) {
    if let Some(init) = local.init {
        // inrelevant and inserted by the compiler
        let expr = strip_drop_temps(init);

        if let ExprKind::MethodCall(method, receiver, args, _) = &expr.kind
            && is_idempotent(method.ident.name)
        {
            check_method_call(cx, expr, method, receiver, args, map);
            // record the new binding if it is a simple identifier
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                map.insert(hir_id, (method.ident.name, args));
            }
        } else {
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                // try to inherit the alias symbol otherwise we check the expr
                if let (retval, _) = try_inherit_alias(expr, hir_id, map)
                    && !retval
                {
                    if let Some((symbol, args)) = check_expr(cx, expr, map) {
                        map.insert(hir_id, (symbol, args));
                    }
                }
            } else {
                check_expr(cx, expr, map);
            }
        }
    }

    if let Some(els) = local.els {
        walk_block(cx, els, map);
    }
}

fn is_idempotent(name: Symbol) -> bool {
    matches!(
        name,
        sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs | sym::floor | sym::max
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
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
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
        ExprKind::Cast(expr, _) => check_expr(cx, expr, map),
        ExprKind::DropTemps(inner) => check_expr(cx, inner, map),
        ExprKind::Unary(_, expr) => check_expr(cx, expr, map),
        ExprKind::Repeat(expr, _) => check_expr(cx, expr, map),
        ExprKind::Let(let_expr) => check_expr(cx, let_expr.init, map),
        ExprKind::AddrOf(_, _, expr) => check_expr(cx, expr, map),
        ExprKind::Index(arr, idx, _) => {
            check_expr(cx, arr, map);
            check_expr(cx, idx, map);
            None
        },
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
fn check_func_args<'tcx>(
    cx: &LateContext<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
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
fn are_args_equal<'tcx>(
    cx: &LateContext<'tcx>,
    args1: &'tcx [rustc_hir::Expr<'tcx>],
    args2: &'tcx [rustc_hir::Expr<'tcx>],
) -> bool {
    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
        let const_context = ConstEvalCtxt::new(cx);
        if let Some(const_1) = const_context.eval(arg1)
            && let Some(const_2) = const_context.eval(arg2)
        {
            if const_1 != const_2 {
                return false;
            }
        } else if let Some(id1) = path_to_local(arg1)
            && let Some(id2) = path_to_local(arg2)
        {
            if id1 != id2 {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn check_method_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    method: &rustc_hir::PathSegment<'tcx>,
    receiver: &'tcx rustc_hir::Expr<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {

    let peeled_receiver = strip_drop_temps(receiver);

    // recurse into receiver only if it's not itself a method call
    if !matches!(peeled_receiver.kind, ExprKind::MethodCall(..)) {
        check_expr(cx, receiver, map);
    }

    // invalidate args mutable
    check_func_args(cx, args, map);

    // if the receiver is borrowed with a "&mut self"
    if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
        let fn_sig = cx.tcx.fn_sig(def_id).skip_binder();
        if let Some(first_ty) = fn_sig.inputs().skip_binder().get(0) {
            // to get the self
            if let rustc_middle::ty::Ref(_, _, Mutability::Mut) = first_ty.kind() {
                invalidate_left_value(peeled_receiver, map);
            }
        }
    }

    if is_idempotent(method.ident.name)
        && let Some(hir_id) = path_to_local(peeled_receiver)
        && let Some((recorded_method, recorded_args)) = map.get(&hir_id)
    {
        if *recorded_method == method.ident.name && are_args_equal(cx, recorded_args, args) {
            span_lint(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
            );
        } else {
            map.insert(hir_id, (method.ident.name, args));
            return Some((method.ident.name, args));
        }
    } else if is_idempotent(method.ident.name)
        && let ExprKind::MethodCall(recursive_method, _, recv_args, _) = peeled_receiver.kind
    {
        if method.ident.name == recursive_method.ident.name
            && are_args_equal(cx, recv_args, args)
        {
            span_lint(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
            );
        }
    } else if is_idempotent(method.ident.name) {
        return Some((method.ident.name, args));
    }
    None
}

fn check_if<'tcx>(
    cx: &LateContext<'tcx>,
    then_block: &'tcx rustc_hir::Expr<'tcx>,
    else_block: &Option<&'tcx rustc_hir::Expr<'tcx>>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
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
            map.retain(|hir_id, (method, args)| {
                if let Some((obtained_method, obtained_args)) = then_map.get(hir_id)
                    && let Some((obtained_method2, obtained_args2)) = else_map.get(hir_id)
                {
                    return obtained_method == method
                        && are_args_equal(cx, args, obtained_args)
                        && obtained_method2 == method
                        && are_args_equal(cx, args, obtained_args2);
                }
                false
            });
        } else {
            //"else if" or other node diff than block,
            // clear everything that the then branch might have changed
            map.retain(|hir_id, (method, args)| {
                if let Some((obtained_method, obtained_args)) = then_map.get(hir_id) {
                    return obtained_method == method && are_args_equal(cx, args, obtained_args);
                }
                false
            });
            check_expr(cx, else_expr, map);
        }
    } else {
        // no "else" branch
        map.retain(|hir_id, (method, args)| {
            if let Some((obtained_method, obtained_args)) = then_map.get(hir_id) {
                return obtained_method == method && are_args_equal(cx, args, obtained_args);
            }
            false
        });
    }
    None
}

fn check_assign<'tcx>(
    cx: &LateContext<'tcx>,
    left_value: &'tcx rustc_hir::Expr<'tcx>,
    right_value: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    if let Some(dst_hir_id) = path_to_local(left_value) {
        if let (retval, _) = try_inherit_alias(right_value, dst_hir_id, map)
            && !retval
        {
            if let Some((symbol, args)) = check_expr(cx, right_value, map) {
                map.insert(dst_hir_id, (symbol, args));
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
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    let mut loop_map = map.clone();
    walk_block(cx, block, &mut loop_map);

    // A state survives only if it existed before the loop and if it was left completely identical
    // inside the loop body.
    map.retain(|hir_id, (method, args)| {
        if let Some((obtained_method, obtained_args)) = loop_map.get(hir_id) {
            return obtained_method == method && are_args_equal(cx, args, obtained_args);
        }
        false
    });
    None
}

fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    arms: &'tcx [rustc_hir::Arm<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
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
    map.retain(|hir_id, (method, args)| {
        arm_maps.iter().all(|arm_map| {
            if let Some((obtained_method, obtained_args)) = arm_map.get(hir_id) {
                return obtained_method == method && are_args_equal(cx, args, obtained_args);
            }
            false
        })
    });

    None
}

fn check_closure<'tcx>(
    cx: &LateContext<'tcx>,
    closure: &'tcx rustc_hir::Closure<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &[rustc_hir::Expr<'_>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    // invalidate any variables captured mutably
    for capture in cx.typeck_results().closure_min_captures_flattened(closure.def_id) {
        if capture.mutability == Mutability::Mut {
            map.shift_remove(&capture.get_root_variable());
        }
    }
    None
}

fn try_inherit_alias<'a>(
    expr: &rustc_hir::Expr<'_>,
    pat_hir_id: HirId,
    map: &mut FxIndexMap<HirId, (Symbol, &'a [rustc_hir::Expr<'a>])>,
) -> (bool, &'a [rustc_hir::Expr<'a>]) {
    if let Some(src_hir_id) = path_to_local(expr)
        && let Some(&(symbol, args)) = map.get(&src_hir_id)
    {
        map.insert(pat_hir_id, (symbol, args));
        (true, args)
    } else {
        (false, &[])
    }
}

fn strip_drop_temps<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>) -> &'tcx rustc_hir::Expr<'tcx> {
    let mut e = expr;
    while let ExprKind::DropTemps(inner) = &e.kind {
        e = inner;
    }
    e
}
