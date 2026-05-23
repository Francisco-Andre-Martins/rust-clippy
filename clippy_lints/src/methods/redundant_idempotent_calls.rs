use rustc_hir::{Body, FnDecl, intravisit::FnKind};
use rustc_lint::LateContext;
use rustc_span::Span;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    walk_block(cx, &body.value);
}


use super::REDUNDANT_IDEMPOTENT_CALLS;

pub(super) fn check_fn<'tcx>{
    cx: $LateContext<'tcx>,
    kind: FnKind<'tcx>,
    decl: &'tcx FnDecl<'tcx>,
    body: &'tcx Body<'tcx>,
    span: Span
} {
    todo!()
}