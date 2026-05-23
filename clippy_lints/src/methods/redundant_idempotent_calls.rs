use clippy_utils::sym;
use rustc_lint::LateContext;
use rustc_middle::mir::Body;
use rustc_mir_dataflow::JoinSemiLattice;
use rustc_span::Symbol;

use super::REDUNDANT_IDEMPOTENT_CALLS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    // Avoids ICE
    if fn_has_unsatisfiable_preds(cx, body.source.def_id()) {
        return;
    }

    // State initilizer, every local mapping to Bottom
    let mut state: IndexVec<Local, MethodState> = IndexVec::from_elem(
        MethodState::Bottom,
        &body.local_decls,
    );

    // Iterate over basic blocks in order
    for (_bb, bbdata) in body.basic_blocks.iter_enumerated() {

        for statement in &bbdata.statements{

        }

        let terminator = bbdata.terminator();
    }

}


#[derive(Clone, Debug, PartialEq, Eq)]
enum MethodState {
    Bottom,
    Applied(Symbol),
    Top,
}

// Logic:
// update if needed the join value
// https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_mir_dataflow/framework/lattice.rs.html#107-121
// e.g.
// self=Top, other=Applied(to_lowercase) ---> no change is needed, it remains Top
impl JoinSemiLattice for MethodState {
    fn join(&mut self, other: &Self) -> bool {
        let result = match (&*self, other) {
            (Self::Top, _) | (_, Self::Bottom) => return false,
            (Self::Applied(a), Self::Applied(b)) if a == b => return false,
            (Self::Bottom, Self::Applied(x)) => Self::Applied(x.clone()),
            _=> Self::Top,
        };
        // self has changed
        *self = result;
        true
    }
}

fn is_idempotent(name: Symbol) -> bool {
    matches!(name,
        sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs
    )
}
