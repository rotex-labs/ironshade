use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::operations::{Result, Scratch, capture, run, sha256_file, workspace};

#[derive(Deserialize)]
pub(crate) struct Manifest {
    pub(crate) slang: Slang,
    validators: Validators,
    pub(crate) licenses: Vec<License>,
}

#[derive(Deserialize)]
pub(crate) struct Slang {
    pub(crate) version: String,
    pub(crate) commit: String,
    linux: Release,
    windows: Release,
}

#[derive(Deserialize)]
struct Release {
    url: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Validators {
    naga: String,
    spirv_tools: String,
    spirv_headers: String,
    glslang: String,
}

#[derive(Deserialize)]
pub(crate) struct License {
    pub(crate) path: String,
    pub(crate) spdx: String,
    pub(crate) sha256: String,
}

impl Manifest {
    pub(crate) fn load() -> Result<(Self, String)> {
        let contents = fs::read_to_string(workspace().join("conformance/manifest.toml"))?;
        let identity = format!("{:x}", Sha256::digest(contents.as_bytes()));
        Ok((toml::from_str(&contents)?, identity))
    }
}

#[derive(Clone, Copy)]
enum Platform {
    Linux,
    Windows,
}

impl Platform {
    fn current() -> Result<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Ok(Self::Linux),
            ("windows", "x86_64") => Ok(Self::Windows),
            (os, arch) => Err(format!("oracle provisioning is unavailable on {os}/{arch}").into()),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Linux => "linux-x86_64",
            Self::Windows => "windows-x86_64",
        }
    }
}

pub(crate) struct Installed {
    root: PathBuf,
    platform: Platform,
}

impl Installed {
    pub(crate) fn slangc(&self) -> PathBuf {
        let executable = match self.platform {
            Platform::Linux => "slangc",
            Platform::Windows => "slangc.exe",
        };
        self.root.join("slang/bin").join(executable)
    }

    pub(crate) fn validator(&self, name: &str) -> Option<PathBuf> {
        matches!(self.platform, Platform::Linux).then(|| self.root.join("bin").join(name))
    }

    fn has_executables(&self) -> bool {
        self.slangc().is_file()
            && ["naga", "spirv-val", "glslangValidator"]
                .iter()
                .filter_map(|name| self.validator(name))
                .all(|path| path.is_file())
    }

    fn complete(&self, identity: &str) -> bool {
        self.has_executables()
            && fs::read_to_string(self.root.join("complete")).is_ok_and(|actual| actual == identity)
    }
}

fn cache(manifest_id: &str, platform: Platform) -> Installed {
    Installed {
        root: workspace()
            .join("target/ironshade-oracles")
            .join(format!("{}-{manifest_id}", platform.id())),
        platform,
    }
}

pub(crate) fn installed(manifest_id: &str) -> Result<Installed> {
    let cache = cache(manifest_id, Platform::current()?);
    if !cache.complete(manifest_id) {
        return Err("oracle cache is incomplete. Run cargo xtask fetch-oracles".into());
    }
    Ok(cache)
}

pub(crate) fn fetch() -> Result<()> {
    let (manifest, identity) = Manifest::load()?;
    let platform = Platform::current()?;
    let cache = cache(&identity, platform);
    if cache.complete(&identity) {
        println!("Oracle tools are ready at {}", cache.root.display());
        return Ok(());
    }
    if cache.root.exists() {
        // Only a completed directory is published below. An existing incomplete
        // directory means files were removed after installation.
        return Err(format!(
            "incomplete oracle cache at {}. Remove that cache entry and retry",
            cache.root.display()
        )
        .into());
    }

    let parent = cache.root.parent().ok_or("oracle cache has no parent")?;
    let work = Scratch::new(parent, "install")?;
    let staging = Installed {
        root: work.path().join("installed"),
        platform,
    };
    install_slang(&manifest.slang, &staging, work.path())?;
    if matches!(platform, Platform::Linux) {
        install_validators(&manifest.validators, &staging, work.path())?;
    }
    if !staging.has_executables() {
        return Err("oracle installation is missing executables".into());
    }
    fs::write(staging.root.join("complete"), &identity)?;
    if let Err(error) = fs::rename(&staging.root, &cache.root)
        && !cache.complete(&identity)
    {
        return Err(error.into());
    }
    println!("Oracle tools installed at {}", cache.root.display());
    Ok(())
}

fn install_slang(slang: &Slang, staging: &Installed, work: &Path) -> Result<()> {
    println!("Installing slangc {}", slang.version);
    let (release, archive_name) = match staging.platform {
        Platform::Linux => (&slang.linux, "slang.tar.gz"),
        Platform::Windows => (&slang.windows, "slang.zip"),
    };
    let archive = work.join(archive_name);
    run(Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "2",
            "--max-time",
            "300",
            "--output",
        ])
        .arg(&archive)
        .arg(&release.url))?;
    let actual = sha256_file(&archive)?;
    if actual != release.sha256 {
        return Err(format!(
            "slangc archive SHA-256 mismatch: expected {}, got {actual}",
            release.sha256
        )
        .into());
    }
    let destination = staging.root.join("slang");
    fs::create_dir_all(&destination)?;
    run(Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&destination))?;
    let version = Command::new(staging.slangc()).arg("-version").output()?;
    let stdout = String::from_utf8_lossy(&version.stdout);
    let stderr = String::from_utf8_lossy(&version.stderr);
    if !version.status.success()
        || ![stdout.trim(), stderr.trim()].contains(&slang.version.as_str())
    {
        return Err(format!("unexpected slangc version: {stdout}{stderr}").into());
    }
    Ok(())
}

fn install_validators(pins: &Validators, staging: &Installed, work: &Path) -> Result<()> {
    println!("Building spirv-val and glslangValidator from pinned sources");
    let spirv = work.join("spirv-tools");
    checkout(
        "https://github.com/KhronosGroup/SPIRV-Tools.git",
        &pins.spirv_tools,
        &spirv,
    )?;
    checkout(
        "https://github.com/KhronosGroup/SPIRV-Headers.git",
        &pins.spirv_headers,
        &spirv.join("external/spirv-headers"),
    )?;
    let glslang = work.join("glslang");
    checkout(
        "https://github.com/KhronosGroup/glslang.git",
        &pins.glslang,
        &glslang,
    )?;
    build(
        &spirv,
        "spirv-val",
        &["-DSPIRV_SKIP_TESTS=ON", "-DSPIRV_WERROR=OFF"],
    )?;
    build(
        &glslang,
        "glslang-standalone",
        &["-DENABLE_OPT=OFF", "-DGLSLANG_TESTS=OFF"],
    )?;

    let bin = staging.root.join("bin");
    fs::create_dir_all(&bin)?;
    fs::copy(spirv.join("build/tools/spirv-val"), bin.join("spirv-val"))?;
    fs::copy(
        glslang.join("build/StandAlone/glslang"),
        bin.join("glslangValidator"),
    )?;

    println!("Installing naga-cli {}", pins.naga);
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run(Command::new(cargo)
        .args([
            "install",
            "--locked",
            "--version",
            &format!("={}", pins.naga),
            "--root",
        ])
        .arg(&staging.root)
        .args(["--target-dir"])
        .arg(work.join("naga-build"))
        .arg("naga-cli"))?;
    Ok(())
}

fn checkout(url: &str, commit: &str, path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    run(Command::new("git")
        .current_dir(path)
        .args(["init", "--quiet"]))?;
    run(Command::new("git").current_dir(path).args([
        "fetch",
        "--quiet",
        "--depth=1",
        url,
        commit,
    ]))?;
    run(Command::new("git").current_dir(path).args([
        "checkout",
        "--quiet",
        "--detach",
        "FETCH_HEAD",
    ]))?;
    let actual = capture(
        Command::new("git")
            .current_dir(path)
            .args(["rev-parse", "HEAD"]),
    )?;
    if actual.trim() != commit {
        return Err(format!("unexpected revision for {url}: {}", actual.trim()).into());
    }
    Ok(())
}

fn build(source: &Path, target: &str, options: &[&str]) -> Result<()> {
    let build = source.join("build");
    run(Command::new("cmake")
        .arg("-S")
        .arg(source)
        .arg("-B")
        .arg(&build)
        .args([
            "-G",
            "Ninja",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=OFF",
        ])
        .args(options))?;
    run(Command::new("cmake").arg("--build").arg(&build).args([
        "--parallel",
        "4",
        "--target",
        target,
    ]))?;
    Ok(())
}
