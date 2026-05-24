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
        // what is this hasmap for storing?
        let mut map = FxHashMap::default();
        walk_block(cx, block, &mut map);
    }
}
// note to self, method calls are expressions, not statements
// wait, an if is an expression too??? the hell
fn walk_block<'tcx>(cx: &LateContext<'tcx>, block: &'tcx rustc_hir::Block<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    for stmt in block.stmts {
        match &stmt.kind {
            StmtKind::Let(local) => check_let(local, map),
            StmtKind::Semi(expr) => {
                check_expr(cx, expr, map);                
            },
            StmtKind::Expr(expr)=>{
                check_expr(cx, expr, map);
            }
            _ => {},
        }
    }

    if let Some(expr) = block.expr {
        check_expr(cx, expr, map);
    }
}

fn invalidate_left_value<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>, map: &mut FxHashMap<HirId, Symbol>) {
    if  let Some(hir_id) = path_to_local(expr)
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
    match expr.kind{
        // TODO check if args have mut reference to any variable in map!!
        ExprKind::MethodCall(method, receiver, args, _) =>{
            if  args.is_empty()
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
        ExprKind::If(_,is_a_block,maybe_block)=>{
            match is_a_block.kind{
                ExprKind::Block(block,..)=>{
                    walk_block(cx, &block, map);
                }
                _=>{}
            }
            match maybe_block{
                Some(stuff)=>{
                    match stuff.kind{
                        ExprKind::Block(block,..)=>{
                            walk_block(cx, &block, map);
                        }
                        _=>{}
                    }
                }
                _=>{}
            }
            
        }
        ExprKind::Assign(left_value,right_value,..)=>{
            check_expr(cx,right_value,map);
            invalidate_left_value(left_value, map);
        }
        ExprKind::AssignOp(_op,left_value,right_value)=>{
            check_expr(cx,right_value,map);
            invalidate_left_value(left_value, map);
        }
        ExprKind::Loop(block,..)=>{
            walk_block(cx,block,map);
        }
        _=>{}
    }

    
}
