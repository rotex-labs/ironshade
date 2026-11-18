//! An external backend exercising the public HAL on an empty module.

use ironshade_target::artifact::{Payload, Reflection};
use ironshade_target::base::{Diagnostic, Diagnostics};
use ironshade_target::caps::{Cap, CapabilityTable, RecipeId, Support};
use ironshade_target::{
    Emitter, LegalModule, Legalizer, Module, Pass, PayloadKind, Profile, SourceEmitter, SourceText,
    Target, TargetId,
};

/// A profile for the illustrative backend.
pub const PROFILE: Profile = Profile {
    representation: "example-text",
    environment: "test",
};

/// An example target with an optional final legality failure.
#[derive(Default)]
pub struct ExampleTarget {
    /// Forces the final legality check to return a diagnostic.
    pub reject_legalization: bool,
    #[cfg(test)]
    trace: Option<tests::Trace>,
}

/// Final legality check for the example.
pub struct ExampleLegalizer {
    reject: bool,
    passes: Vec<Box<dyn Pass>>,
}

impl Legalizer for ExampleLegalizer {
    fn passes(&mut self) -> &mut [Box<dyn Pass>] {
        &mut self.passes
    }

    fn check_legal(&self, _: &Module) -> Result<(), Diagnostics> {
        if self.reject {
            return Err(vec![Diagnostic::error(
                "example.illegal",
                "example rejected the module",
                None,
            )]);
        }
        Ok(())
    }
}

/// Illustrates source emission through the HAL.
pub struct ExampleEmitter;

impl Emitter for ExampleEmitter {
    type Output = SourceText;

    fn emit(&mut self, _: &LegalModule<'_>, _: &Reflection) -> Result<Self::Output, Diagnostics> {
        Ok(SourceText("example module\n".to_owned()))
    }
}

impl SourceEmitter for ExampleEmitter {}

impl Target for ExampleTarget {
    type Legalizer = ExampleLegalizer;
    type Emitter = ExampleEmitter;
    type Options = ();

    fn id(&self) -> TargetId {
        TargetId {
            namespace: "example",
            name: "text",
            revision: 1,
        }
    }

    fn profiles(&self) -> &[Profile] {
        &[PROFILE]
    }

    fn payload_kind(&self, _: &Profile) -> PayloadKind {
        PayloadKind::Source
    }

    fn capabilities(&self, _: &Profile, _: &()) -> CapabilityTable {
        let mut table = CapabilityTable::default();
        table.insert(
            Cap::PushConstants,
            Support::Lowered(RecipeId::from("example.push")),
        );
        table.insert(
            Cap::PointSize,
            Support::Emulated(RecipeId::from("example.sprite")),
        );
        table.insert(
            Cap::Float64,
            Support::Unsupported("example representation is 32-bit".to_owned()),
        );
        table
    }

    fn legalizer(&self, _: &Profile, _: &()) -> Self::Legalizer {
        #[cfg(test)]
        let passes = self
            .trace
            .iter()
            .map(|trace| Box::new(tests::Record("target", trace.clone())) as Box<dyn Pass>)
            .collect();
        #[cfg(not(test))]
        let passes = Vec::new();
        ExampleLegalizer {
            reject: self.reject_legalization,
            passes,
        }
    }

    fn plan_abi(&self, _: &LegalModule<'_>, _: &Profile) -> Result<Reflection, Diagnostics> {
        Ok(Reflection::default())
    }

    fn emitter(&self, _: &Profile, _: &()) -> Self::Emitter {
        ExampleEmitter
    }

    fn self_check(&self, payload: &Payload, _: &Profile) -> Result<(), Diagnostics> {
        match payload {
            Payload::Source(text) if text.ends_with('\n') => Ok(()),
            _ => Err(vec![Diagnostic::error(
                "example.payload",
                "invalid example payload",
                None,
            )]),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Arc, Mutex};

    use ironshade_target::{CompileRequest, caps::Requirement, compile};

    use super::*;

    pub(super) type Trace = Arc<Mutex<Vec<&'static str>>>;

    fn request<'a>(
        requirements: &'a [Requirement],
        approvals: &'a BTreeSet<RecipeId>,
    ) -> CompileRequest<'a, ()> {
        CompileRequest {
            profile: &PROFILE,
            options: &(),
            module: Module::empty(),
            requirements,
            approved_emulations: approvals,
        }
    }

    #[test]
    fn rejects_requirements_and_final_illegality() {
        let target = ExampleTarget::default();
        for (capability, code) in [
            (Cap::SeparateSampler, "capability.unknown"),
            (Cap::Float64, "capability.unsupported"),
            (Cap::PointSize, "capability.emulation_denied"),
            (Cap::PushConstants, "target.missing_recipe"),
        ] {
            let result = compile(
                &target,
                request(
                    &[Requirement {
                        capability,
                        origin: None,
                    }],
                    &BTreeSet::new(),
                ),
                &mut BTreeMap::new(),
            );
            assert_eq!(result.unwrap_err()[0].code, code);
        }
        let result = compile(
            &ExampleTarget {
                reject_legalization: true,
                ..ExampleTarget::default()
            },
            request(&[], &BTreeSet::new()),
            &mut BTreeMap::new(),
        );
        assert_eq!(result.unwrap_err()[0].code, "example.illegal");
    }

    pub(super) struct Record(pub(super) &'static str, pub(super) Trace);

    impl Pass for Record {
        fn name(&self) -> &'static str {
            self.0
        }
        fn run(&mut self, _: &mut Module) -> Result<(), Diagnostics> {
            self.1.lock().unwrap().push(self.0);
            Ok(())
        }
    }

    #[test]
    fn records_requirements_and_emits_deterministically() {
        let trace = Trace::default();
        let mut recipes: BTreeMap<RecipeId, Box<dyn Pass>> = ["example.push", "example.sprite"]
            .into_iter()
            .map(|id| {
                (
                    RecipeId::from(id),
                    Box::new(Record(id, trace.clone())) as Box<dyn Pass>,
                )
            })
            .collect();
        let requirements =
            [Cap::PushConstants, Cap::PushConstants, Cap::PointSize].map(|capability| {
                Requirement {
                    capability,
                    origin: None,
                }
            });
        let approvals = BTreeSet::from([RecipeId::from("example.sprite")]);
        let target = ExampleTarget {
            trace: Some(trace.clone()),
            ..ExampleTarget::default()
        };
        let a = compile(&target, request(&requirements, &approvals), &mut recipes).unwrap();
        let b = compile(&target, request(&requirements, &approvals), &mut recipes).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            *trace.lock().unwrap(),
            [
                "example.push",
                "example.sprite",
                "target",
                "example.push",
                "example.sprite",
                "target"
            ]
        );
        assert_eq!(a.reflection.capability_report.len(), 3);
        assert_eq!(a.payload, Payload::Source("example module\n".to_owned()));
    }
}
