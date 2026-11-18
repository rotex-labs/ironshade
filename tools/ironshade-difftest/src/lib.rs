//! Run the pinned slangc oracle and normalize reflection for the Nova fixtures.

mod reflection;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use ironshade_artifact::Reflection;
pub use ironshade_base::ShaderStage;
use serde::Serialize;

/// Error raised by the oracle process or its outputs.
pub type Error = Box<dyn std::error::Error>;

/// The M0 oracle targets.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Vulkan SPIR-V with the Rotex compiler flags.
    Spirv,
    /// WGSL with the Rotex preprocessor definition.
    Wgsl,
}

impl Target {
    /// Target spelling for slangc and stable case IDs.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Spirv => "spirv",
            Self::Wgsl => "wgsl",
        }
    }

    /// Validator executable for this target.
    pub const fn validator(self) -> &'static str {
        match self {
            Self::Spirv => "spirv-val",
            Self::Wgsl => "naga",
        }
    }
}

/// One source, entry point, stage, and target.
pub struct Case<'a> {
    /// Source file path.
    pub source: &'a Path,
    /// Entry point to compile.
    pub entry: &'a str,
    /// Shader stage.
    pub stage: ShaderStage,
    /// Oracle target.
    pub target: Target,
}

/// Normalized evidence produced by an oracle run.
#[derive(Debug, Serialize)]
pub struct Snapshot {
    /// Pinned slangc version.
    pub slangc_version: String,
    /// Fixture file name.
    pub source: String,
    /// Entry point name.
    pub entry: String,
    /// Destination representation.
    pub target: Target,
    /// Arguments that affect output.
    pub flags: Vec<String>,
    /// Header version of the emitted SPIR-V module.
    pub spirv_version: Option<String>,
    /// Whether the emitted WGSL contains an immediate variable.
    pub wgsl_uses_immediates: Option<bool>,
    /// Validator disposition and executable.
    pub validation: Validation,
    /// Normalized reflection.
    pub reflection: Reflection,
}

/// Recorded result of validating the oracle payload.
#[derive(Debug, Serialize)]
pub struct Validation {
    /// Validator name.
    pub tool: &'static str,
    /// Whether the oracle payload passed the validator.
    pub accepted: bool,
}

/// Runs the oracle, writes raw evidence in the output directory, and returns its snapshot.
pub fn run_oracle(
    case: &Case<'_>,
    slangc: &Path,
    version: &str,
    validator: &Path,
    output: &Path,
) -> Result<Snapshot, Error> {
    let source = case
        .source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("fixture must have a UTF-8 file name")?;
    fs::create_dir_all(output)?;
    let reflection_path = output.join("reflection.raw.json");
    let payload = output.join(match case.target {
        Target::Spirv => "shader.spv",
        Target::Wgsl => "shader.wgsl",
    });
    let mut flags = vec![
        "-matrix-layout-column-major",
        "-fvk-use-entrypoint-name",
        "-fvk-use-gl-layout",
        "-O2",
    ];
    match case.target {
        Target::Spirv => flags.extend(["-profile", "spirv_1_0"]),
        Target::Wgsl => flags.push("-DROTEX_WGSL=1"),
    }

    let result = Command::new(slangc)
        .arg(case.source)
        .args([
            "-entry",
            case.entry,
            "-stage",
            case.stage.as_str(),
            "-target",
            case.target.name(),
        ])
        .args(&flags)
        .arg("-reflection-json")
        .arg(&reflection_path)
        .arg("-o")
        .arg(&payload)
        .output()?;
    write_log(output, "slangc", &result)?;
    if !result.status.success() {
        return Err(format!(
            "{source}/{} {}: slangc failed ({}): {}",
            case.entry,
            case.target.name(),
            result.status,
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    let reflection = reflection::normalize(&fs::read(reflection_path)?, case.entry, case.stage)?;
    let bytes = fs::read(&payload)?;
    if bytes.is_empty() {
        return Err("slangc produced an empty payload".into());
    }
    let (spirv_version, wgsl_uses_immediates) = match case.target {
        Target::Spirv => (Some(spirv_version(&bytes)?), None),
        Target::Wgsl => (
            None,
            Some(std::str::from_utf8(&bytes)?.contains("var<immediate>")),
        ),
    };

    let mut command = Command::new(validator);
    if matches!(case.target, Target::Spirv) {
        command.args(["--target-env", "vulkan1.1"]);
    }
    let result = command.arg(&payload).output()?;
    let tool = case.target.validator();
    write_log(output, tool, &result)?;
    if result.status.code().is_none() {
        return Err(format!("{tool} terminated without an exit code").into());
    }
    Ok(Snapshot {
        slangc_version: version.to_owned(),
        source: source.to_owned(),
        entry: case.entry.to_owned(),
        target: case.target,
        flags: flags.into_iter().map(str::to_owned).collect(),
        spirv_version,
        wgsl_uses_immediates,
        validation: Validation {
            tool,
            accepted: result.status.success(),
        },
        reflection,
    })
}

fn write_log(directory: &Path, prefix: &str, result: &Output) -> std::io::Result<()> {
    fs::write(directory.join(format!("{prefix}.stdout")), &result.stdout)?;
    fs::write(directory.join(format!("{prefix}.stderr")), &result.stderr)
}

fn spirv_version(bytes: &[u8]) -> Result<String, Error> {
    if bytes.len() < 20 || bytes[..4] != [3, 2, 35, 7] {
        return Err("invalid SPIR-V header in oracle payload".into());
    }
    Ok(format!("{}.{}", bytes[6], bytes[5]))
}
