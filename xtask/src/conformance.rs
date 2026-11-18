use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::operations::{Result, capture, sha256_file, workspace};
use crate::oracles::Manifest;

pub(crate) fn run(dry_run: bool) -> Result<()> {
    let (manifest, _) = Manifest::load()?;
    let root = workspace().join("conformance/slang-tests");
    let tests = root.join("tests");
    if !root.join(".git").exists() || !tests.is_dir() {
        return Err(
            "initialize the corpus with git submodule update --init conformance/slang-tests".into(),
        );
    }
    let head = capture(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root),
    )?;
    if head.trim() != manifest.slang.commit {
        return Err(format!(
            "Slang corpus revision {} does not match {}",
            head.trim(),
            manifest.slang.commit
        )
        .into());
    }

    let mut files = Vec::new();
    walk(&tests, &mut files)?;
    files.sort();
    audit(&root, &files, &manifest)?;
    if dry_run {
        let report = ironshade_conformance::classify(&tests, &files, &manifest.slang.commit)?;
        let output = workspace().join("target/ironshade-conformance");
        fs::create_dir_all(&output)?;
        let mut json = serde_json::to_string_pretty(&report)?;
        json.push('\n');
        fs::write(output.join("report.json"), json)?;
        fs::write(output.join("index.html"), report.html())?;
        println!(
            "Classified {} directives in {} source files.",
            report.directives.len(),
            report.source_files
        );
        println!("Reports: {}", output.display());
        if let Some(invalid) = report.directives.iter().find(|directive| {
            directive.applicability == ironshade_conformance::Applicability::Invalid
        }) {
            return Err(format!(
                "could not classify {} at {}:{}",
                invalid.directive, invalid.path, invalid.line
            )
            .into());
        }
    }
    Ok(())
}

fn walk(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), files)?;
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn audit(root: &Path, files: &[PathBuf], manifest: &Manifest) -> Result<()> {
    let mut expected = BTreeMap::new();
    for license in &manifest.licenses {
        if license.spdx.trim().is_empty()
            || expected.insert(Path::new(&license.path), license).is_some()
        {
            return Err(
                format!("invalid or duplicate license decision for {}", license.path).into(),
            );
        }
    }

    let root_license = root.join("LICENSE");
    for path in std::iter::once(&root_license).chain(files.iter().filter(|file| is_license(file))) {
        let relative = path.strip_prefix(root)?;
        let license = expected.remove(relative).ok_or_else(|| {
            format!(
                "license file {} has no recorded decision",
                relative.display()
            )
        })?;
        let actual = sha256_file(path)?;
        if actual != license.sha256 {
            return Err(format!(
                "license file {} changed since its audit",
                relative.display()
            )
            .into());
        }
    }
    if !expected.is_empty() {
        return Err(format!("license decisions without files: {:?}", expected.keys()).into());
    }
    println!(
        "Slang corpus license audit passed ({} files).",
        manifest.licenses.len()
    );
    Ok(())
}

fn is_license(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let upper = name.to_ascii_uppercase();
            ["LICENSE", "LICENCE", "COPYING", "NOTICE"]
                .iter()
                .any(|base| {
                    upper
                        .strip_prefix(base)
                        .is_some_and(|tail| tail.is_empty() || tail.starts_with(['.', '-', '_']))
                })
        })
}
