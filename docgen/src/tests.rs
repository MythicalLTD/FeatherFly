use crate::html::{self, PageContext};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
struct TestEntry {
    crate_name: String,
    module: String,
    name: String,
}

#[derive(Debug, Default)]
struct TestRunSummary {
    passed: usize,
    failed: usize,
    ignored: usize,
    ran: bool,
    output: String,
}

pub fn generate_test_docs(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    let inventory = collect_inventory();
    let summary = run_tests_if_requested();

    html::write_page(
        &output.join("index.html"),
        "Unit tests",
        PageContext::tests("tests"),
        &html::PageMeta::new(
            "FeatherFly unit test inventory and CI results.",
            "tests/index.html",
        )
        .with_source("CI workflow", ".github/workflows/ci.yml"),
        &page_body(&inventory, &summary),
    )
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn crate_from_running_line(line: &str) -> Option<String> {
    let start = line.find("deps/")? + 5;
    let rest = &line[start..];
    let end = rest.find('-')?;
    Some(rest[..end].replace('_', "-"))
}

fn collect_inventory() -> Vec<TestEntry> {
    let output = Command::new("cargo")
        .args(["test", "--workspace", "--", "--list", "--format", "terse"])
        .current_dir(workspace_root())
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    let mut current_crate = String::from("featherfly");

    for line in text.lines() {
        if line.contains("Running unittests") {
            if let Some(crate_name) = crate_from_running_line(line) {
                current_crate = normalize_crate_name(&crate_name);
            }
            continue;
        }

        let Some((qualified, _)) = line.split_once(": test") else {
            continue;
        };

        let parts: Vec<_> = qualified.split("::").collect();
        if parts.len() < 2 {
            continue;
        }

        let name = parts.last().unwrap().to_string();
        let module = parts[..parts.len() - 1].join("::");
        entries.push(TestEntry {
            crate_name: current_crate.clone(),
            module,
            name,
        });
    }

    entries.sort_by(|a, b| {
        (&a.crate_name, &a.module, &a.name).cmp(&(&b.crate_name, &b.module, &b.name))
    });
    entries
}

fn normalize_crate_name(raw: &str) -> String {
    match raw {
        "featherfly" => "featherfly".into(),
        "generate-docs" | "generate_docs" => "featherfly (generate-docs)".into(),
        "featherfly-docgen" | "featherfly_docgen" => "featherfly-docgen".into(),
        "featherfly-plugin-sdk" | "featherfly_plugin_sdk" => "featherfly-plugin-sdk".into(),
        other => other.replace('_', "-"),
    }
}

fn extract_count(line: &str, label: &str) -> usize {
    for part in line.split(';') {
        let part = part.trim();
        let Some(idx) = part.find(label) else {
            continue;
        };
        let before = part[..idx].trim();
        if let Some(num) = before.split_whitespace().last()
            && let Ok(n) = num.parse()
        {
            return n;
        }
    }
    0
}

fn parse_counts(line: &str) -> Option<(usize, usize, usize)> {
    if !line.contains("test result:") {
        return None;
    }
    Some((
        extract_count(line, " passed"),
        extract_count(line, " failed"),
        extract_count(line, " ignored"),
    ))
}

fn run_tests_if_requested() -> TestRunSummary {
    if std::env::var("DOCS_RUN_TESTS").ok().as_deref() != Some("1") {
        return TestRunSummary::default();
    }

    let output = Command::new("cargo")
        .args(["test", "--workspace"])
        .current_dir(workspace_root())
        .output();

    let Ok(output) = output else {
        return TestRunSummary {
            ran: true,
            output: "Failed to execute cargo test".into(),
            ..Default::default()
        };
    };

    let text = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    let mut summary = TestRunSummary {
        ran: true,
        output: text.clone(),
        ..Default::default()
    };

    for line in text.lines() {
        if !line.contains("test result:") {
            continue;
        }
        let Some((passed, failed, ignored)) = parse_counts(line) else {
            continue;
        };
        summary.passed += passed;
        summary.failed += failed;
        summary.ignored += ignored;
    }

    summary
}

fn page_body(inventory: &[TestEntry], summary: &TestRunSummary) -> String {
    let mut status = String::new();
    if summary.ran {
        let status_text = if summary.failed > 0 {
            "Failures detected"
        } else if summary.passed == 0 {
            "No tests recorded"
        } else {
            "All passed"
        };
        status.push_str(&format!(
            "<h2>Last run</h2>
<div class=\"meta-grid\">
  <div class=\"meta-item\"><div class=\"meta-label\">Passed</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Failed</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Ignored</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Status</div><div class=\"meta-value\">{}</div></div>
</div>",
            summary.passed, summary.failed, summary.ignored, status_text,
        ));
        if !summary.output.is_empty() {
            status.push_str(&format!(
                "<details><summary>Raw output</summary><div class=\"details-body\"><pre><code>{}</code></pre></div></details>",
                html::html_escape(&summary.output)
            ));
        }
    } else {
        status.push_str(
            "<p>Test results are captured when docs are built with <code>DOCS_RUN_TESTS=1</code> (CI deploy workflow). Run <code>make ci</code> locally.</p>",
        );
    }

    let mut table = String::from(
        "<table><thead><tr><th>Crate</th><th>Module</th><th>Test</th></tr></thead><tbody>",
    );
    for entry in inventory {
        table.push_str(&format!(
            "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>",
            html::html_escape(&entry.crate_name),
            html::html_escape(&entry.module),
            html::html_escape(&entry.name),
        ));
    }
    table.push_str("</tbody></table>");

    format!(
        "{header}
<p>Inventory of workspace unit tests. Generated from <code>cargo test --workspace -- --list</code>.</p>
{status}
<h2>Test inventory ({count})</h2>
{table}",
        header = html::page_header(
            "Unit tests",
            "Automated tests run in CI for FeatherFly and the plugin SDK.",
        ),
        count = inventory.len(),
        status = status,
        table = table,
    )
}
