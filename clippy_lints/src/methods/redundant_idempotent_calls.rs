use clippy_utils::diagnostics::span_lint;
use clippy_utils::sym;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::Res;
use rustc_hir::{Body, ExprKind, HirId, PatKind, QPath, StmtKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;

use super::REDUNDANT_IDEMPOTENT_CALLS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    // If the body of the funciton is a block
    if let ExprKind::Block(block, _) = &body.value.kind {
        let mut map = FxHashMap::default();
        walk_block(cx, block, &mut map);
    }
}

fn walk_block<'tcx>(cx: &LateContext<'tcx>, block: &'tcx rustc_hir::Block<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    for stmt in block.stmts {
        match &stmt.kind {
            StmtKind::Let(local) => check_let(local, map),
            StmtKind::Semi(expr) => {
                invalidate_map(expr, map);
                check_expr(cx, expr, map);
            },
            _ => {},
        }
    }

    if let Some(expr) = block.expr {
        check_expr(cx, expr, map);
    }
}

fn invalidate_map<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    if let ExprKind::Assign(lhs, _, _) = &expr.kind
        && let Some(hir_id) = path_to_local(lhs)
    {
        map.remove(&hir_id);
    }
}

fn check_let<'tcx>(local: &'tcx rustc_hir::LetStmt<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    if let Some(init) = local.init
        && let ExprKind::MethodCall(method, _receiver, args, _) = &init.kind
        && args.is_empty()
        && is_idempotent(method.ident.name)
        && let PatKind::Binding(_, hir_id, _, _) = local.pat.kind
    {
        map.insert(hir_id, method.ident.name);
    }
}

fn is_idempotent(name: Symbol) -> bool {
    matches!(name, sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs | sym::floor)
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

fn check_expr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    if let ExprKind::MethodCall(method, receiver, args, _) = &expr.kind
        && args.is_empty()
        && is_idempotent(method.ident.name)
        && let Some(hir_id) = path_to_local(receiver)
        && let Some(recorded_method) = map.get(&hir_id)
        && *recorded_method == method.ident.name
    {
        span_lint(
            cx,
            REDUNDANT_IDEMPOTENT_CALLS,
            expr.span,
            "redundant call to idempotent method, the result is already the same",
        );
    }
}
