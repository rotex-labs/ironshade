use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use ironshade_difftest::{Case, ShaderStage, Target, run_oracle};
use serde::Deserialize;

use crate::operations::{Result, Scratch, capture, workspace};
use crate::oracles::{self, Manifest};

#[derive(Deserialize)]
struct Jobs {
    shader: Vec<Shader>,
}

#[derive(Deserialize)]
struct Shader {
    file: String,
    stages: Vec<ShaderStage>,
}

pub(crate) fn run(fixtures: &Path, update: bool) -> Result<()> {
    let (manifest, identity) = Manifest::load()?;
    let installed = oracles::installed(&identity)?;
    let jobs: Jobs = toml::from_str(&fs::read_to_string(fixtures.join("jobs.toml"))?)?;
    let spirv_val = installed
        .validator("spirv-val")
        .ok_or("the Nova oracle baseline runs on Linux x86-64")?;
    let naga = installed
        .validator("naga")
        .ok_or("the Nova oracle baseline runs on Linux x86-64")?;
    let glslang = installed
        .validator("glslangValidator")
        .ok_or("the Nova oracle baseline runs on Linux x86-64")?;
    let output = Scratch::new(&workspace().join("target/ironshade-difftest"), "run")?.keep();
    println!("Raw oracle evidence: {}", output.display());

    // Exercise the pinned ESSL validator independently. Nova's baseline jobs
    // are SPIR-V and WGSL and therefore do not go through glslang.
    let glsl = output.join("essl-smoke.frag");
    fs::write(
        &glsl,
        "#version 300 es\nprecision mediump float;\nout vec4 color;\nvoid main() { color = vec4(1.0); }\n",
    )?;
    capture(Command::new(glslang).arg(glsl))?;

    let mut current = BTreeMap::new();
    for shader in jobs.shader {
        let path = Path::new(&shader.file);
        if path.file_name() != Some(path.as_os_str())
            || path.extension().is_none_or(|ext| ext != "slang")
            || shader.stages.is_empty()
        {
            return Err(format!("invalid shader job {}", shader.file).into());
        }
        let source = fixtures.join(path);
        let stem = shader.file.trim_end_matches(".slang");
        for stage in shader.stages {
            let entry = format!("{}Main", stage.as_str());
            for (target, validator) in [(Target::Spirv, &spirv_val), (Target::Wgsl, &naga)] {
                let id = format!("{stem}-{entry}-{}", target.name());
                if current.contains_key(&id) {
                    return Err(format!("duplicate oracle case {id}").into());
                }
                let snapshot = run_oracle(
                    &Case {
                        source: &source,
                        entry: &entry,
                        stage,
                        target,
                    },
                    &installed.slangc(),
                    &manifest.slang.version,
                    validator,
                    &output.join(&id),
                )?;
                let json = serde_json::to_string_pretty(&snapshot)? + "\n";
                fs::write(output.join(&id).join("normalized.json"), &json)?;
                current.insert(id, json);
            }
        }
    }
    if current.is_empty() {
        return Err("fixture manifest defines no oracle cases".into());
    }

    let snapshots = workspace().join("tests/snapshots/nova");
    if update {
        fs::create_dir_all(&snapshots)?;
        for (id, json) in &current {
            fs::write(snapshots.join(format!("{id}.json")), json)?;
        }
        println!("Updated {} Nova oracle snapshots.", current.len());
    } else {
        let mut changed = Vec::new();
        for (id, json) in &current {
            let expected = snapshots.join(format!("{id}.json"));
            if fs::read_to_string(&expected).ok().as_deref() != Some(json) {
                changed.push(id.to_owned());
            }
        }
        if !changed.is_empty() {
            return Err(format!("Nova snapshots differ or are missing: {}. Inspect the raw evidence and use --update to accept changes", changed.join(", ")).into());
        }
        for entry in fs::read_dir(&snapshots)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
                && !path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|id| current.contains_key(id))
            {
                return Err(format!("Nova snapshot has no oracle case: {}", path.display()).into());
            }
        }
        println!("Verified {} Nova oracle snapshots.", current.len());
    }
    Ok(())
}
