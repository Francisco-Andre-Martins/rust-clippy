use rustc_lint::LateContext;
use rustc_middle::mir::Body;
use rustc_middle::mir::StatementKind::{Assign, StorageLive};
use rustc_middle::mir::Local;
use rustc_middle::mir::BasicBlock;
use rustc_index::IndexVec;
//use rustc_mir_dataflow::JoinSemiLattice;
use std::collections::HashMap;
use rustc_span::Symbol;
use clippy_utils::{fn_has_unsatisfiable_preds, sym};
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
    let mut terminators =  Vec::new();
    //this map keeps the record of the successors for each block, so as to determine the predecessors of each block
    let mut block_successors_map = HashMap::new();
    let mut block_predecessors_map = HashMap::new();
    // Iterate over basic blocks in order
    println!("the entire mir, ugly, {:#?}",body);
    for (_bb, bbdata) in body.basic_blocks.iter_enumerated() {
        let terminator = bbdata.terminator();
        block_successors_map.insert(_bb,terminator.successors());
        terminators.push(terminator);
        for successor in terminator.successors(){
            println!("these are the successors {:#?}", successor);
            if(block_predecessors_map.contains_key(&successor)){
                let mut predecessors: Vec<&BasicBlock>= block_predecessors_map.get(&successor).unwrap_or_else(
                    panic!("you suck")
                );
                predecessors.push(_bb);
            } else{
                let mut predecessors = Vec::new();
                predecessors.push(_bb);
                block_predecessors_map.insert(successor,predecessors);
            }
            
        }
        //update existing symbolds given the predecessors
        println!("this is the current block {:#?}",_bb);
        //update/create new symbols according to the current statements
        for statement in &bbdata.statements{
            match &statement.kind{
                Assign(box (place, rvalue))=>{
                    println!("this is an assignment, you suck {:#?} {:#?}",place,rvalue);
                }
                StorageLive(sucks)=>{
                    println!("this is a storage live {:#?}",sucks);
                }
                _ =>{
                    println!("dont care");
                }
            }
        }
        

        
        println!("this the terminator {:#?}",terminator);
    }

}


#[derive(Clone, Debug, PartialEq, Eq)]
enum MethodState {
    Bottom,
    Applied(Symbol),
    Top,
}

// // Logic:
// // update if needed the join value
// // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_mir_dataflow/framework/lattice.rs.html#107-121
// // e.g.
// // self=Top, other=Applied(to_lowercase) ---> no change is needed, it remains Top
// impl JoinSemiLattice for MethodState {
//     fn join(&mut self, other: &Self) -> bool {
//         let result = match (&*self, other) {
//             (Self::Top, _) | (_, Self::Bottom) => return false,
//             (Self::Applied(a), Self::Applied(b)) if a == b => return false,
//             (Self::Bottom, Self::Applied(x)) => Self::Applied(x.clone()),
//             _=> Self::Top,
//         };
//         // self has changed
//         *self = result;
//         true
//     }
// }

fn is_idempotent(name: Symbol) -> bool {
    matches!(name,
        sym::to_lowercase | sym::to_uppercase | sym::trim | sym::abs
    )
}
