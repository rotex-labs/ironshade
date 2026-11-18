//! Shared graph pass contract for target-neutral and target-specific legalization.

use ironshade_base::Diagnostics;
use ironshade_ir::Module;

/// A graph pass run against a mutable module.
pub trait Pass {
    /// Stable name used in diagnostics and pass traces.
    fn name(&self) -> &'static str;

    /// Runs this pass and reports failures as diagnostics.
    fn run(&mut self, module: &mut Module) -> Result<(), Diagnostics>;
}
