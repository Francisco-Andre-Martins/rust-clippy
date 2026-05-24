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

fn walk_block<'tcx>(cx: &LateContext<'tcx>, block: &'tcx rustc_hir::Block<'tcx>, map: &mut FxIndexMap<HirId, Symbol>) {
    for stmt in block.stmts {
        //println!("blcok is {:#?}",stmt.kind);
        match &stmt.kind {
            
            StmtKind::Let(local) => check_let(cx, local, map),
            StmtKind::Semi(expr) | StmtKind::Expr(expr)  => {
                check_expr(cx, expr, map);
            },
            _ => {},
        }
    }

    if let Some(expr) = block.expr {
        check_expr(cx, expr, map);
    }
}

fn invalidate_left_value<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>, map: &mut FxIndexMap<HirId, Symbol>) {
    if let Some(hir_id) = path_to_local(expr) {
        map.shift_remove(&hir_id);
    }
}

fn check_let<'tcx>(cx: &LateContext<'tcx>, local: &'tcx rustc_hir::LetStmt<'tcx>, map: &mut FxIndexMap<HirId, Symbol>) {
    if let Some(init) = local.init
        && let ExprKind::MethodCall(method, receiver, args, _) = &init.kind
        {
            check_method_call(cx,init,method,receiver,args,map);
            // record the new binding if it is a simple identifier
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                map.insert(hir_id, method.ident.name);
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
) -> bool {
    match &expr.kind {
        ExprKind::MethodCall(method, receiver, args, _) => check_method_call(cx, expr, method, receiver, args, map),
        ExprKind::If(_, then_block, else_block) => check_if(cx, then_block, else_block, map),
        ExprKind::Assign(left_value, right_value, _) => check_assign(cx, left_value, right_value, map),
        ExprKind::AssignOp(_, left_value, right_value) => check_assign(cx, left_value, right_value, map),
        ExprKind::Loop(block, _, _, _) => check_loop(cx, block, map),
        ExprKind::Match(_, arms, _) => check_match(cx, arms, map),
        _ => false,
    }
}

fn check_method_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    method: &rustc_hir::PathSegment<'tcx>,
    receiver: &'tcx rustc_hir::Expr<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
    if !args.is_empty() {
        // invalidade any locals passed as &mut
        for arg in args {
            if let ExprKind::AddrOf(_, Mutability::Mut, inner) = &arg.kind
                && let Some(hir_id) = path_to_local(inner)
            {
                map.shift_remove(&hir_id);
            }
        }

        return false;
    }
    //println!("the receiver is...{:#?}",receiver.kind);

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
            return true;
        }
    }
    else if is_idempotent(method.ident.name)  && let ExprKind::MethodCall(recursive_method,..)=receiver.kind
    {
        //println!("the og name is {:#?}, the upwards is {:#?}",method.ident.name,recursive_method.ident.name);
        if method.ident.name == recursive_method.ident.name{
            span_lint(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
            );
        }
    }
    false
}

fn check_if<'tcx>(
    cx: &LateContext<'tcx>,
    then_block: &'tcx rustc_hir::Expr<'tcx>,
    else_block: &Option<&'tcx rustc_hir::Expr<'tcx>>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
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
    false
}

fn check_assign<'tcx>(
    cx: &LateContext<'tcx>,
    left_value: &'tcx rustc_hir::Expr<'tcx>,
    right_value: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
    if !check_expr(cx, right_value, map) {
        invalidate_left_value(left_value, map);
    } else if let Some(hir_id) = path_to_local(right_value) {
        
        //map.insert(hir_id,);
    }
    false
}

fn check_loop<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
    walk_block(cx, block, map);
    false
}

fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    arms: &'tcx [rustc_hir::Arm<'tcx>],
    map: &mut FxIndexMap<HirId, Symbol>,
) -> bool {
    for arm in arms {
        let mut arm_map = map.clone();
        if let ExprKind::Block(block, _) = &arm.body.kind {
            walk_block(cx, block, &mut arm_map);
        }
    }
    false
}
