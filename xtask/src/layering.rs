use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

const BACKENDS: &[&str] = &[
    "ironshade-spirv",
    "ironshade-wgsl",
    "ironshade-glsl",
    "ironshade-hlsl",
    "ironshade-dxbc",
    "ironshade-d3d9",
    "ironshade-dxil",
    "ironshade-bridge-dxc",
];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    pkg: String,
}

pub(crate) fn check() -> Result<(), String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--all-features",
            "--locked",
        ])
        .output()
        .map_err(|error| format!("could not run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }

    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("could not read cargo metadata: {error}"))?;
    let violations = metadata.violations();
    if violations.is_empty() {
        return Ok(());
    }
    Err(format!("dependency violations:\n{}", violations.join("\n")))
}

impl Metadata {
    fn violations(&self) -> Vec<String> {
        let packages: HashMap<_, _> = self
            .packages
            .iter()
            .map(|package| (package.id.as_str(), package))
            .collect();
        let edges: HashMap<_, _> = self
            .resolve
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node.deps.as_slice()))
            .collect();
        let mut violations = Vec::new();

        for package in &self.packages {
            let direct = edges.get(package.id.as_str()).copied().unwrap_or_default();
            for dependency in direct {
                let dependency = &packages[dependency.pkg.as_str()].name;
                let reason = if dependency.starts_with("rotex-") {
                    "Rotex dependencies are forbidden"
                } else if package.name == "ironshade-artifact"
                    && !matches!(dependency.as_str(), "ironshade-base" | "serde")
                {
                    "artifact dependencies are limited to ironshade-base and serde"
                } else {
                    continue;
                };
                violations.push(format!("{} -> {dependency}: {reason}", package.name));
            }

            if self.directory(package) != Some("crates") {
                continue;
            }
            let mut visited = HashSet::from([package.id.as_str()]);
            let mut pending: Vec<_> = direct.iter().collect();
            while let Some(edge) = pending.pop() {
                let id = edge.pkg.as_str();
                if !visited.insert(id) {
                    continue;
                }
                let dependency = packages[id];
                if let Some(next) = edges.get(id) {
                    pending.extend(next.iter());
                }
                let is_frontend = matches!(
                    dependency.name.as_str(),
                    "ironshade-syntax" | "ironshade-sema" | "ironshade-graph"
                );
                let reason = if matches!(self.directory(dependency), Some("tools" | "xtask")) {
                    "compiler crates cannot depend on repository tooling"
                } else if BACKENDS.contains(&package.name.as_str()) && is_frontend {
                    "backends cannot depend on frontends"
                } else if matches!(
                    (package.name.as_str(), dependency.name.as_str()),
                    ("ironshade-graph", "ironshade-sema") | ("ironshade-sema", "ironshade-graph")
                ) {
                    "the text and graph frontends must be independent"
                } else {
                    continue;
                };
                violations.push(format!("{} -> {}: {reason}", package.name, dependency.name));
            }
        }

        violations.sort_unstable();
        violations.dedup();
        violations
    }

    fn directory<'a>(&self, package: &'a Package) -> Option<&'a str> {
        package
            .manifest_path
            .strip_prefix(&self.workspace_root)
            .ok()?
            .iter()
            .next()?
            .to_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direct_and_transitive_layering_violations() {
        let metadata = Metadata {
            packages: [
                ("crates/ironshade-spirv", "ironshade-spirv"),
                ("crates/ironshade-target", "ironshade-target"),
                ("crates/ironshade-sema", "ironshade-sema"),
                ("crates/ironshade-graph", "ironshade-graph"),
                ("crates/ironshade-artifact", "ironshade-artifact"),
                ("crates/ironshade-base", "ironshade-base"),
                ("tools/ironshade-difftest", "ironshade-difftest"),
                ("vendor/rotex-types", "rotex-types"),
            ]
            .into_iter()
            .map(|(directory, name)| Package {
                id: name.into(),
                name: name.into(),
                manifest_path: PathBuf::from(directory).join("Cargo.toml"),
            })
            .collect(),
            resolve: Resolve {
                nodes: [
                    ("ironshade-spirv", &["ironshade-target"][..]),
                    (
                        "ironshade-target",
                        &["ironshade-sema", "ironshade-difftest"],
                    ),
                    ("ironshade-sema", &["ironshade-graph"]),
                    (
                        "ironshade-artifact",
                        &["ironshade-base", "ironshade-target"],
                    ),
                    ("ironshade-difftest", &["rotex-types"]),
                ]
                .into_iter()
                .map(|(name, dependencies)| Node {
                    id: name.into(),
                    deps: dependencies
                        .iter()
                        .map(|dependency| Dependency {
                            pkg: (*dependency).into(),
                        })
                        .collect(),
                })
                .collect(),
            },
            workspace_root: PathBuf::new(),
        };

        assert_eq!(
            metadata.violations(),
            [
                "ironshade-artifact -> ironshade-difftest: compiler crates cannot depend on repository tooling",
                "ironshade-artifact -> ironshade-target: artifact dependencies are limited to ironshade-base and serde",
                "ironshade-difftest -> rotex-types: Rotex dependencies are forbidden",
                "ironshade-sema -> ironshade-graph: the text and graph frontends must be independent",
                "ironshade-spirv -> ironshade-difftest: compiler crates cannot depend on repository tooling",
                "ironshade-spirv -> ironshade-graph: backends cannot depend on frontends",
                "ironshade-spirv -> ironshade-sema: backends cannot depend on frontends",
                "ironshade-target -> ironshade-difftest: compiler crates cannot depend on repository tooling",
            ]
        );
    }
}
