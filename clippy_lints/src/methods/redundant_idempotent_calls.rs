use rustc_lint::LateContext;
use rustc_middle::mir::Body;
use rustc_mir_dataflow::JoinSemiLattice;;
use rustc_span::Symbol;



use std::collections::HashMap;

use super::REDUNDANT_IDEMPOTENT_CALLS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    
}


#[derive(Clone, Debug, Eq )]
enum MehtodState {
    Bottom,
    Applied(Symbol),
    Top,
}