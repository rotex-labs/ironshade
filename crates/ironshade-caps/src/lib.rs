//! Capability vocabulary and legalization planning.

use std::collections::{BTreeMap, BTreeSet};

use ironshade_base::{Diagnostic, Diagnostics, Span};

/// A shader capability required by an input module.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Cap {
    /// Push constant block access.
    PushConstants,
    /// Vertex point size.
    PointSize,
    /// Read and write access to structured buffers.
    StructuredBufferReadWrite,
    /// Separately declared texture and sampler.
    SeparateSampler,
    /// Double-precision floating point.
    Float64,
    /// A backend-defined namespaced capability.
    Extension(String),
}

impl Cap {
    /// Stable spelling used in reports and diagnostics.
    pub fn name(&self) -> &str {
        match self {
            Self::PushConstants => "resource.push_constants",
            Self::PointSize => "builtin.point_size",
            Self::StructuredBufferReadWrite => "resource.structured_buffer.rw",
            Self::SeparateSampler => "resource.separate_sampler",
            Self::Float64 => "type.f64",
            Self::Extension(name) => name,
        }
    }
}

/// Stable identifier for a named legalization recipe.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecipeId(pub String);

impl From<&str> for RecipeId {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

/// One condition required by a shader, with its origin if available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    /// Required capability.
    pub capability: Cap,
    /// Origin of the requirement.
    pub origin: Option<Span>,
}

/// How a profile can satisfy a requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Support {
    /// Directly supported.
    Native,
    /// Needs a semantics-preserving recipe.
    Lowered(RecipeId),
    /// Needs an explicitly approved emulation recipe.
    Emulated(RecipeId),
    /// Cannot meet the requirement.
    Unsupported(String),
    /// A necessary fact is unavailable.
    Unknown,
}

/// Support declared by a target for one profile. Missing entries resolve to Unknown.
#[derive(Clone, Debug, Default)]
pub struct CapabilityTable {
    entries: BTreeMap<Cap, Support>,
}

impl CapabilityTable {
    /// Declares support for a capability.
    pub fn insert(&mut self, capability: Cap, support: Support) {
        self.entries.insert(capability, support);
    }

    /// Gets the declared support, including the default for a missing entry.
    pub fn support(&self, capability: &Cap) -> &Support {
        self.entries.get(capability).unwrap_or(&Support::Unknown)
    }

    /// Resolves all requirements or returns a diagnostic for each rejected requirement.
    pub fn plan(
        &self,
        requirements: &[Requirement],
        approved_emulations: &BTreeSet<RecipeId>,
    ) -> Result<Vec<Resolution>, Diagnostics> {
        let mut resolved = Vec::with_capacity(requirements.len());
        let mut diagnostics = Vec::new();
        for requirement in requirements {
            let capability = &requirement.capability;
            let support = self.support(capability);
            let rejection = match support {
                Support::Unknown => Some(("capability.unknown", "support is unknown".to_owned())),
                Support::Unsupported(reason) => Some(("capability.unsupported", reason.clone())),
                Support::Emulated(recipe) if !approved_emulations.contains(recipe) => Some((
                    "capability.emulation_denied",
                    format!("emulation recipe {} requires policy approval", recipe.0),
                )),
                _ => None,
            };
            if let Some((code, reason)) = rejection {
                diagnostics.push(Diagnostic::error(
                    code,
                    format!("{}: {reason}", capability.name()),
                    requirement.origin,
                ));
            } else {
                resolved.push(Resolution {
                    requirement: requirement.clone(),
                    support: support.clone(),
                });
            }
        }
        if diagnostics.is_empty() {
            Ok(resolved)
        } else {
            Err(diagnostics)
        }
    }
}

/// Planned support for a requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    /// The original requirement.
    pub requirement: Requirement,
    /// The planned way to satisfy it.
    pub support: Support,
}
