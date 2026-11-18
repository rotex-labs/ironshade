//! Classification of directives from the pinned Slang compiler test suite.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;

// Pinned slang-test's endsWithAllowedExtension list. The runner reads all of
// these, including shader sources embedded in documentation.
const EXTENSIONS: &[&str] = &[
    ".slang.md",
    ".slang",
    ".hlsl",
    ".fx",
    ".glsl",
    ".vert",
    ".frag",
    ".geom",
    ".tesc",
    ".tese",
    ".comp",
    ".internal",
    ".ahit",
    ".chit",
    ".miss",
    ".rgen",
    ".c",
    ".cpp",
    ".cu",
];

/// A file or test directive, with its location and classification.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Directive {
    /// Slash-delimited path relative to the corpus root.
    pub path: String,
    /// One-based source line.
    pub line: usize,
    /// Directive name as written upstream.
    pub directive: String,
    /// Upstream test command, if present.
    pub test_type: Option<String>,
    /// Upstream command options, if present.
    pub options: Option<String>,
    /// CLI arguments as written, for reporting only.
    pub args: Option<String>,
    /// File and directive categories in sorted order.
    pub categories: Vec<String>,
    /// Whether or how this test can take part in compile-only comparisons.
    pub applicability: Applicability,
}

/// Classification for the Ironshade GPU-free harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    /// Directive controls file-level test discovery.
    FileDirective,
    /// Test disabled upstream.
    Disabled,
    /// Test skipped by a file-level ignore directive.
    IgnoredFile,
    /// Test can be compared by compilation.
    CompileOnly,
    /// Compute test whose compilation can be compared separately.
    ComputeCompileOnly,
    /// Requires an API, stage or runner outside the initial compiler targets.
    NotApplicable,
    /// Test-like directive with syntax the classifier cannot parse.
    Invalid,
}

impl Applicability {
    fn label(self) -> &'static str {
        match self {
            Self::FileDirective => "file_directive",
            Self::Disabled => "disabled",
            Self::IgnoredFile => "ignored_file",
            Self::CompileOnly => "compile_only",
            Self::ComputeCompileOnly => "compute_compile_only",
            Self::NotApplicable => "not_applicable",
            Self::Invalid => "invalid",
        }
    }
}

/// Deterministic dry-run inventory and counts. Categories can overlap.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Pinned commit being classified.
    pub commit: String,
    /// Number of upstream source files inspected.
    pub source_files: usize,
    /// Number of recorded directives of each kind.
    pub by_directive: BTreeMap<String, usize>,
    /// Number of tests for each upstream test command.
    pub by_type: BTreeMap<String, usize>,
    /// Number of tests in each category.
    pub by_category: BTreeMap<String, usize>,
    /// Number of directives in each applicability class.
    pub by_applicability: BTreeMap<String, usize>,
    /// Source records in stable path and line order.
    pub directives: Vec<Directive>,
}

/// Scans the test tree and summarizes every recognized directive without executing it.
pub fn classify(root: &Path, files: &[std::path::PathBuf], commit: &str) -> io::Result<Report> {
    let mut report = Report {
        commit: commit.to_owned(),
        source_files: 0,
        by_directive: BTreeMap::new(),
        by_type: BTreeMap::new(),
        by_category: BTreeMap::new(),
        by_applicability: BTreeMap::new(),
        directives: Vec::new(),
    };
    for path in files {
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        let file_name = relative
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        if !EXTENSIONS
            .iter()
            .any(|extension| file_name.ends_with(extension))
        {
            continue;
        }
        report.source_files += 1;
        let bytes = fs::read(path)?;
        for directive in classify_source(&file_name, &String::from_utf8_lossy(&bytes)) {
            increment(&mut report.by_directive, &directive.directive);
            increment(
                &mut report.by_applicability,
                directive.applicability.label(),
            );
            if let Some(test_type) = &directive.test_type {
                increment(&mut report.by_type, test_type);
                if directive.categories.is_empty() {
                    increment(&mut report.by_category, "(uncategorized)");
                }
                for category in &directive.categories {
                    increment(&mut report.by_category, category);
                }
            }
            report.directives.push(directive);
        }
    }
    report
        .directives
        .sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    Ok(report)
}

fn increment(counts: &mut BTreeMap<String, usize>, label: &str) {
    *counts.entry(label.to_owned()).or_default() += 1;
}

fn classify_source(path: &str, source: &str) -> Vec<Directive> {
    let mut directives = Vec::new();
    let mut current_categories = BTreeSet::new();
    let mut ignored = false;
    for (index, line) in source.lines().enumerate() {
        let Some(comment) = line.trim_start().strip_prefix("//") else {
            continue;
        };
        let comment = comment.trim_start_matches('/').trim_start();
        let boundary = comment
            .find(|ch: char| !ch.is_ascii_uppercase() && ch != '_')
            .unwrap_or(comment.len());
        let (kind, tail) = comment.split_at(boundary);
        if !matches!(
            kind,
            "TEST"
                | "DISABLE_TEST"
                | "DIAGNOSTIC_TEST"
                | "DISABLE_DIAGNOSTIC_TEST"
                | "TEST_CATEGORY"
                | "TEST_IGNORE_FILE"
        ) {
            continue;
        }
        // Ignore prose containing TEST at the start of a comment.
        if !tail.trim_start().starts_with(['(', ':']) && !tail.trim().is_empty() {
            continue;
        }
        let mut record = Directive {
            path: path.to_owned(),
            line: index + 1,
            directive: kind.to_owned(),
            test_type: None,
            options: None,
            args: None,
            categories: Vec::new(),
            applicability: Applicability::FileDirective,
        };
        match kind {
            "TEST_IGNORE_FILE" => ignored = true,
            "TEST_CATEGORY" => match parentheses(tail.trim_start()) {
                Some((labels, _)) => {
                    current_categories.extend(categories(labels));
                    record.categories.extend(current_categories.iter().cloned());
                }
                None => record.applicability = Applicability::Invalid,
            },
            _ => match test_fields(tail.trim_start()) {
                Some((labels, test_type, options, args)) => {
                    let mut all_categories = current_categories.clone();
                    all_categories.extend(categories(labels));
                    record.categories.extend(all_categories);
                    record.test_type = Some(test_type.to_owned());
                    record.options = options.map(str::to_owned);
                    record.args = Some(args.to_owned());
                    record.applicability = if kind.starts_with("DISABLE_") {
                        Applicability::Disabled
                    } else {
                        applicability(test_type, args, &record.categories)
                    };
                }
                None => record.applicability = Applicability::Invalid,
            },
        }
        directives.push(record);
    }
    if ignored {
        for directive in &mut directives {
            if directive.test_type.is_some() && directive.applicability != Applicability::Disabled {
                directive.applicability = Applicability::IgnoredFile;
            }
        }
    }
    directives
}

fn categories(labels: &str) -> impl Iterator<Item = String> + '_ {
    labels
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parentheses(text: &str) -> Option<(&str, &str)> {
    text.strip_prefix('(')?.split_once(')')
}

fn test_fields(mut tail: &str) -> Option<(&str, &str, Option<&str>, &str)> {
    let mut labels = "";
    if tail.starts_with('(') {
        (labels, tail) = parentheses(tail)?;
    }
    let specification = tail.trim_start().strip_prefix(':')?.trim_start();
    let delimiter = specification.find(['(', ':'])?;
    let (name, tail) = specification.split_at(delimiter);
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let (opts, rest) = if tail.starts_with('(') {
        let (opts, rest) = parentheses(tail)?;
        (Some(opts.trim()), rest)
    } else {
        (None, tail)
    };
    Some((
        labels,
        name,
        opts,
        rest.trim_start().strip_prefix(':')?.trim(),
    ))
}

fn applicability(test_type: &str, args: &str, categories: &[String]) -> Applicability {
    let unsupported = [
        "cuda",
        "optix",
        "metal",
        "d3d11",
        "d3d12",
        "dxbc",
        "dxil",
        "ray-tracing",
    ];
    if categories
        .iter()
        .any(|category| unsupported.contains(&category.as_str()))
    {
        return Applicability::NotApplicable;
    }
    let mut tokens = args.split_ascii_whitespace();
    while let Some(token) = tokens.next() {
        let unsupported_arg = match token {
            "-target" => !matches!(tokens.next(), Some("spirv" | "spirv-asm" | "wgsl")),
            "-stage" => !matches!(tokens.next(), Some("vertex" | "fragment" | "compute")),
            _ => unsupported.contains(&token.trim_start_matches('-')),
        };
        if unsupported_arg {
            return Applicability::NotApplicable;
        }
    }
    match test_type {
        "SIMPLE" | "SIMPLE_EX" | "SIMPLE_LINE" | "REFLECTION" => Applicability::CompileOnly,
        "COMPARE_COMPUTE" | "COMPARE_COMPUTE_EX" | "COMPARE_RENDER_COMPUTE" => {
            Applicability::ComputeCompileOnly
        }
        _ => Applicability::NotApplicable,
    }
}

impl Report {
    /// Renders a standalone HTML count summary.
    pub fn html(&self) -> String {
        let mut html = String::from(
            "<!doctype html>\n<html lang=\"en\"><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width\">\n<title>Ironshade Slang test inventory</title>\n<style>body{font:1rem system-ui,sans-serif;max-width:64rem;margin:2rem auto;padding:0 1rem;color-scheme:light dark}table{border-collapse:collapse;width:min(100%,36rem)}th,td{padding:.3rem .6rem;border-bottom:1px solid GrayText;text-align:left}td:last-child,th:last-child{text-align:right}</style>\n<h1>Slang compiler test inventory</h1>\n",
        );
        let _ = writeln!(
            html,
            "<p>Pinned revision <code>{}</code>. Inspected {} source files and recorded {} directives. Categories can overlap.</p>",
            escape(&self.commit),
            self.source_files,
            self.directives.len()
        );
        for (heading, counts) in [
            ("Applicability", &self.by_applicability),
            ("Test type", &self.by_type),
            ("Category", &self.by_category),
            ("Directive", &self.by_directive),
        ] {
            let _ = writeln!(
                html,
                "<h2>{heading}</h2><table><thead><tr><th scope=\"col\">Name</th><th scope=\"col\">Count</th></tr></thead><tbody>"
            );
            for (key, total) in counts {
                let _ = writeln!(html, "<tr><td>{}</td><td>{total}</td></tr>", escape(key));
            }
            html.push_str("</tbody></table>\n");
        }
        html.push_str("</html>\n");
        html
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_multiple_directives_and_inherits_file_categories() {
        let records = classify_source(
            "test.slang",
            r#"
            // TEST_CATEGORY(compute, vulkan)
            ///TEST(extra):COMPARE_COMPUTE_EX(filecheck-buffer=CHECK):-vk -compute
            // DISABLE_DIAGNOSTIC_TEST:SIMPLE(diag=CHECK):foo.slang
            // TEST:SIMPLE:-target cuda
            // TEST as prose
        "#,
        );
        assert_eq!(records.len(), 4);
        assert_eq!(records[1].categories, ["compute", "extra", "vulkan"]);
        assert_eq!(records[1].test_type.as_deref(), Some("COMPARE_COMPUTE_EX"));
        assert_eq!(records[1].applicability, Applicability::ComputeCompileOnly);
        assert_eq!(records[2].applicability, Applicability::Disabled);
        assert_eq!(records[3].applicability, Applicability::NotApplicable);
    }

    #[test]
    fn reports_ignored_and_malformed_directives() {
        let records = classify_source(
            "test.slang",
            "//TEST:SIMPLE:\n// TEST_IGNORE_FILE:\n//TEST(incomplete\n",
        );
        assert_eq!(records[0].applicability, Applicability::IgnoredFile);
        assert_eq!(records[1].applicability, Applicability::FileDirective);
        assert_eq!(records[2].applicability, Applicability::Invalid);
    }
}
