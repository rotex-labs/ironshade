//! Versioned, backend-independent shader artifacts and reflection.

use ironshade_base::ShaderStage;
use serde::{Deserialize, Serialize};

/// Numeric version of the JSON reflection schema.
pub const SCHEMA_VERSION: u32 = 0;

/// Version marker that rejects unsupported schema versions during deserialization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct SchemaVersion;

impl TryFrom<u32> for SchemaVersion {
    type Error = &'static str;

    fn try_from(version: u32) -> Result<Self, Self::Error> {
        if version == SCHEMA_VERSION {
            Ok(Self)
        } else {
            Err("unsupported Ironshade reflection schema version")
        }
    }
}

impl From<SchemaVersion> for u32 {
    fn from(_: SchemaVersion) -> Self {
        SCHEMA_VERSION
    }
}

/// A compiler output plus the information needed to load it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    /// Target profile that produced this artifact.
    pub target: TargetIdentity,
    /// Emitted code.
    pub payload: Payload,
    /// Entry-point and ABI data.
    pub reflection: Reflection,
}

/// Target and environment identity carried in an artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetIdentity {
    /// Namespace of the backend.
    pub namespace: String,
    /// Name of the backend in its namespace.
    pub name: String,
    /// Revision of the backend contract.
    pub revision: u32,
    /// Output representation, including its version where applicable.
    pub representation: String,
    /// Execution environment, including its version where applicable.
    pub environment: String,
}

/// Emitted shader text or bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Payload {
    /// Source in the target representation.
    Source(String),
    /// Binary in the target representation.
    Binary(Vec<u8>),
}

/// Schema v0 reflection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Reflection {
    /// Version of this reflection document.
    pub schema_version: SchemaVersion,
    /// Entry points and their interfaces.
    pub entry_points: Vec<EntryPoint>,
    /// All declared top-level bindings, including unused declarations.
    pub declared_bindings: Vec<Binding>,
    /// Push and constant blocks with their layouts.
    pub blocks: Vec<Binding>,
    /// Specialization declarations.
    pub specialization_constants: Vec<SpecializationConstant>,
    /// Mapping between logical resources and emitted bindings.
    pub remaps: Vec<BindingRemap>,
    /// Resolution of compilation requirements.
    pub capability_report: Vec<CapabilityResolution>,
}

/// Entry-point-specific reflection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntryPoint {
    /// Entry point name.
    pub name: String,
    /// Shader stage.
    pub stage: ShaderStage,
    /// Compute workgroup size where known.
    pub workgroup_size: Option<[u32; 3]>,
    /// Input interface.
    pub inputs: Vec<InterfaceVariable>,
    /// Output interface.
    pub outputs: Vec<InterfaceVariable>,
    /// Live bindings if determined by the producer.
    pub live_bindings: Option<Vec<BindingUse>>,
}

/// Liveness of one binding for an entry point.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BindingUse {
    /// Name of the binding.
    pub name: String,
    /// Whether the binding is live, when reported.
    pub used: Option<bool>,
}

/// One stage input or output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InterfaceVariable {
    /// Name within the stage interface.
    pub name: String,
    /// Explicit stage I/O location, if known.
    pub location: Option<u32>,
    /// Semantic name, if any.
    pub semantic: Option<String>,
    /// Semantic index, when represented separately.
    pub semantic_index: Option<u32>,
    /// Builtin identity, if reflected.
    pub builtin: Option<String>,
    /// Interpolation mode, if reflected.
    pub interpolation: Option<String>,
    /// Variable type and known layout.
    pub ty: TypeLayout,
}

/// One logical resource or block declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// Resource identity within the reflection.
    pub name: String,
    /// Resource kind.
    pub kind: String,
    /// Access mode, if known.
    pub access: Option<String>,
    /// Array size, if known and fixed.
    pub array_size: Option<u64>,
    /// Source or intermediate binding coordinates.
    pub logical: Option<BindingPoint>,
    /// Binding coordinates in the emitted representation.
    pub physical: Option<BindingPoint>,
    /// Resource type and known layout.
    pub ty: TypeLayout,
}

/// Namespaced coordinates within a binding model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BindingPoint {
    /// Binding class such as descriptor slot or constant register.
    pub class: String,
    /// Descriptor set, group or register space, if known.
    pub space: Option<u32>,
    /// Slot or register index.
    pub index: u32,
}

/// Structural type reflection and known layout measurements.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TypeLayout {
    /// Type kind.
    pub kind: String,
    /// Declared type name, if present.
    pub name: Option<String>,
    /// Scalar representation, if applicable.
    pub scalar: Option<String>,
    /// Element count for fixed-size aggregates, if known.
    pub element_count: Option<u64>,
    /// Matrix row count, if applicable.
    pub rows: Option<u32>,
    /// Matrix column count, if applicable.
    pub columns: Option<u32>,
    /// Matrix major order, if known.
    pub matrix_layout: Option<String>,
    /// Measurements in the producer's reported layout units.
    pub sizes: Vec<LayoutSize>,
    /// Element or result type for arrays, resources and other aggregates.
    pub element: Option<Box<TypeLayout>>,
    /// Struct members in declaration order.
    pub fields: Vec<MemberLayout>,
}

/// Size and alignment reported in one layout class.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LayoutSize {
    /// Layout class, for example uniform bytes.
    pub class: String,
    /// Size, if reported numerically.
    pub size: Option<u64>,
    /// Alignment, if reported.
    pub alignment: Option<u64>,
}

/// Layout of one member relative to its containing type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberLayout {
    /// Member name.
    pub name: String,
    /// Member offset, if reported.
    pub offset: Option<u64>,
    /// Member size, if reported.
    pub size: Option<u64>,
    /// Array or matrix stride, if reported.
    pub stride: Option<u64>,
    /// Member type and nested layout.
    pub ty: TypeLayout,
}

/// One specialization constant declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpecializationConstant {
    /// Specialization constant name.
    pub name: String,
    /// Target specialization identifier.
    pub id: u32,
    /// Scalar type.
    pub ty: String,
}

/// Mapping from one logical resource to a physical binding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BindingRemap {
    /// Resource name.
    pub resource: String,
    /// Emitted binding coordinates.
    pub physical: BindingPoint,
}

/// The successful resolution of one capability requirement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityResolution {
    /// Stable capability name.
    pub capability: String,
    /// How the requirement was met.
    pub resolution: CapabilityMode,
    /// Selected recipe for lowered or emulated capabilities.
    pub recipe: Option<String>,
    /// Original requirement location, if present.
    pub origin: Option<ironshade_base::Span>,
}

/// The ways a compilation can meet a requirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityMode {
    /// Support requires no legalization recipe.
    Native,
    /// A semantics-preserving recipe meets the requirement.
    Lowered,
    /// An explicitly authorized emulation meets the requirement.
    Emulated,
}
