use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// On known idempotent functions (for example floor), it warns a user against using the function twice
    /// ### Why is this bad?
    /// Unnecessary calls may create code confusion and reduce performance
    /// ### Example
    /// ```
    /// let x = 3.14_f32.floor();
    /// x = x.floor(); 
    /// ```
    /// Use instead:
    /// ```
    /// let x = 3.14_f32.floor();
    /// ```
    #[clippy::version = "1.97.0"]
    pub REDUNDANT_IDEMPOTENT_METHODS,
    pedantic,
    "default lint description"
}
declare_lint_pass!(RedundantIdempotentMethods => [REDUNDANT_IDEMPOTENT_METHODS]);

impl LateLintPass<'_> for RedundantIdempotentMethods {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        _: &'tcx Body<'_>,
        _: Span,
        def_id: LocalDefId,
    ) {
        
    }
}
