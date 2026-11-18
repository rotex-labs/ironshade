//! Public backend HAL and the static legalization and emission pipeline.

use std::collections::{BTreeMap, BTreeSet};

pub use ironshade_artifact as artifact;
pub use ironshade_base as base;
pub use ironshade_caps as caps;
pub use ironshade_ir::Module;
pub use ironshade_passes::Pass;
use serde::{Serialize, de::DeserializeOwned};

use artifact::{
    Artifact, CapabilityMode, CapabilityResolution, Payload, Reflection, TargetIdentity,
};
use base::{Diagnostic, Diagnostics};
use caps::{CapabilityTable, RecipeId, Requirement, Support};

/// Major and minor version of the public backend HAL.
pub const HAL_VERSION: (u16, u16) = (0, 1);

/// Stable namespaced identity for a backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetId {
    /// Backend namespace.
    pub namespace: &'static str,
    /// Name in the backend namespace.
    pub name: &'static str,
    /// Backend contract revision.
    pub revision: u32,
}

/// Representation and execution environment for one backend profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    /// Target representation and any relevant version.
    pub representation: &'static str,
    /// Execution environment and any relevant version.
    pub environment: &'static str,
}

/// Whether a backend emits source text or binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadKind {
    /// Source code.
    Source,
    /// Binary code.
    Binary,
}

/// A module that passed the backend's final legalization check.
pub struct LegalModule<'a>(&'a Module);

impl LegalModule<'_> {
    /// Accesses the checked IR.
    pub fn module(&self) -> &Module {
        self.0
    }
}

/// Ordered target passes and the final legality check.
pub trait Legalizer {
    /// Passes to run after shared recipes.
    fn passes(&mut self) -> &mut [Box<dyn Pass>];

    /// Checks that the module satisfies the target's post-legalization invariants.
    fn check_legal(&self, module: &Module) -> Result<(), Diagnostics>;
}

/// Payload generation from checked IR and planned ABI.
pub trait Emitter {
    /// Type of emitted output.
    type Output: Into<Payload>;

    /// Emits the module using the supplied ABI information.
    fn emit(
        &mut self,
        module: &LegalModule<'_>,
        abi: &Reflection,
    ) -> Result<Self::Output, Diagnostics>;
}

/// Source emitted by a backend.
pub struct SourceText(pub String);

impl From<SourceText> for Payload {
    fn from(output: SourceText) -> Self {
        Self::Source(output.0)
    }
}

/// Binary emitted by a backend.
pub struct BinaryBlob(pub Vec<u8>);

impl From<BinaryBlob> for Payload {
    fn from(output: BinaryBlob) -> Self {
        Self::Binary(output.0)
    }
}

/// Marker for emitters producing source.
pub trait SourceEmitter: Emitter<Output = SourceText> {}

/// Marker for emitters producing binary.
pub trait BinaryEmitter: Emitter<Output = BinaryBlob> {}

/// Statically dispatched backend contract.
pub trait Target: Send + Sync + 'static {
    /// Ordered target-specific legalization and final checks.
    type Legalizer: Legalizer;
    /// Emitter for the selected profile.
    type Emitter: Emitter;
    /// Backend options supplied for a request.
    type Options: Default + Clone + Serialize + DeserializeOwned;

    /// Returns the identity of this backend.
    fn id(&self) -> TargetId;

    /// Returns the HAL version implemented by this backend.
    fn hal_version(&self) -> (u16, u16) {
        HAL_VERSION
    }

    /// Profiles that callers can select.
    fn profiles(&self) -> &[Profile];

    /// Payload kind emitted for this profile.
    fn payload_kind(&self, profile: &Profile) -> PayloadKind;

    /// Declares support for capabilities with the given options.
    fn capabilities(&self, profile: &Profile, options: &Self::Options) -> CapabilityTable;

    /// Constructs the target legalizer.
    fn legalizer(&self, profile: &Profile, options: &Self::Options) -> Self::Legalizer;

    /// Plans the physical ABI of the checked module.
    fn plan_abi(
        &self,
        module: &LegalModule<'_>,
        profile: &Profile,
    ) -> Result<Reflection, Diagnostics>;

    /// Constructs the target emitter.
    fn emitter(&self, profile: &Profile, options: &Self::Options) -> Self::Emitter;

    /// Validates target-specific payload invariants.
    fn self_check(&self, payload: &Payload, profile: &Profile) -> Result<(), Diagnostics>;
}

/// Input to the static backend pipeline.
pub struct CompileRequest<'a, Options> {
    /// Selected profile.
    pub profile: &'a Profile,
    /// Backend options.
    pub options: &'a Options,
    /// Initial module.
    pub module: Module,
    /// Capability requirements of the module.
    pub requirements: &'a [Requirement],
    /// Explicitly approved emulation recipes.
    pub approved_emulations: &'a BTreeSet<RecipeId>,
}

/// Runs shared and target legalization, then checks, emits and wraps the payload.
///
/// Requirements are resolved before any pass runs. Shared recipes run once each
/// in first-use order, followed by the legalizer's declared order.
pub fn compile<T: Target>(
    target: &T,
    request: CompileRequest<'_, T::Options>,
    recipes: &mut BTreeMap<RecipeId, Box<dyn Pass>>,
) -> Result<Artifact, Diagnostics> {
    let CompileRequest {
        profile,
        options,
        mut module,
        requirements,
        approved_emulations,
    } = request;
    if target.hal_version().0 != HAL_VERSION.0 || !target.profiles().contains(profile) {
        return Err(vec![Diagnostic::error(
            "target.profile",
            "incompatible HAL or target profile",
            None,
        )]);
    }
    let resolutions = target
        .capabilities(profile, options)
        .plan(requirements, approved_emulations)?;
    let mut order = Vec::new();
    let mut scheduled = BTreeSet::new();
    let mut report = Vec::with_capacity(resolutions.len());
    for resolution in &resolutions {
        let (mode, recipe) = match &resolution.support {
            Support::Native => (CapabilityMode::Native, None),
            Support::Lowered(recipe) => (CapabilityMode::Lowered, Some(recipe)),
            Support::Emulated(recipe) => (CapabilityMode::Emulated, Some(recipe)),
            Support::Unsupported(_) | Support::Unknown => {
                return Err(vec![Diagnostic::error(
                    "target.plan",
                    "unresolved capability in legalization plan",
                    resolution.requirement.origin,
                )]);
            }
        };
        if let Some(recipe) = recipe.filter(|recipe| scheduled.insert(*recipe)) {
            if !recipes.contains_key(recipe) {
                return Err(vec![Diagnostic::error(
                    "target.missing_recipe",
                    format!("missing legalization recipe {}", recipe.0),
                    resolution.requirement.origin,
                )]);
            }
            order.push(recipe);
        }
        report.push(CapabilityResolution {
            capability: resolution.requirement.capability.name().to_owned(),
            resolution: mode,
            recipe: recipe.map(|value| value.0.clone()),
            origin: resolution.requirement.origin,
        });
    }
    for recipe in order {
        recipes
            .get_mut(recipe)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "target.missing_recipe",
                    format!("missing legalization recipe {}", recipe.0),
                    None,
                )]
            })?
            .run(&mut module)?;
    }

    let mut legalizer = target.legalizer(profile, options);
    for pass in legalizer.passes() {
        pass.run(&mut module)?;
    }
    legalizer.check_legal(&module)?;
    let legal = LegalModule(&module);
    let reflection = target.plan_abi(&legal, profile)?;
    let payload = target
        .emitter(profile, options)
        .emit(&legal, &reflection)?
        .into();
    if !matches!(
        (target.payload_kind(profile), &payload),
        (PayloadKind::Source, Payload::Source(_)) | (PayloadKind::Binary, Payload::Binary(_))
    ) {
        return Err(vec![Diagnostic::error(
            "target.payload_kind",
            "emitter returned the wrong payload kind",
            None,
        )]);
    }
    target.self_check(&payload, profile)?;
    Ok(ArtifactWriter::write(
        target.id(),
        *profile,
        payload,
        reflection,
        report,
    ))
}

/// Shared construction of the artifact envelope.
pub struct ArtifactWriter;

impl ArtifactWriter {
    /// Attaches target identity and the complete capability report.
    pub fn write(
        target: TargetId,
        profile: Profile,
        payload: Payload,
        mut reflection: Reflection,
        report: Vec<CapabilityResolution>,
    ) -> Artifact {
        reflection.capability_report = report;
        Artifact {
            target: TargetIdentity {
                namespace: target.namespace.to_owned(),
                name: target.name.to_owned(),
                revision: target.revision,
                representation: profile.representation.to_owned(),
                environment: profile.environment.to_owned(),
            },
            payload,
            reflection,
        }
    }
}
