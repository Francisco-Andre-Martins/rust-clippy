use clippy_utils::sym;
use rustc_hir::{Body, ExprKind, StmtKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;
use std::collections::HashMap;


use super::REDUNDANT_IDEMPOTENT_CALLS

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {

    // If the body of the funciton is a block
    if let ExprKind::Block(block, _) = &body.value.kind {
        let mut map = HashMap::new();
        walk_block(cx, block, &mut map);
    }
}

fn walk_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut HashMap<HirId, Symbol>,
) {
    for stmt in block.stmts {
        match &stmt.kind {
            StmtKind::Let(local) => check_let(local, map),
            StmtKind::Semi(expr) => check_expr(cx, expr, map),
            _ => {},
        }
    }

    if let Some(expr) = block.expr {
        check_expr(cx, expr, map);
    }
}

fn check_let<'tcx>(
    local: &'tcx rustc_hir::LetStmt<'tcx>,
    map: &mut HashMap<HirId, Symbol>,
) {
    if let Some(init) = local.init
        && let ExprKind::MethodCall(mehtod, _receiver, args, _) = &init.kind
        && args.is_empty()
        && is_idempotent(method.ident.name)
        && let PatKind::Binding(_, hir_id, _, _) = local.pat.kind
    {
        map.insert(hir_id, method.ident.name);
    }
}

fn is_idempotent(name: Symbol) -> bool {
    matches!(name,
        sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs
    )
}