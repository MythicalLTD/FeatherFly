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

    html::write(
        &output.join("index.html"),
        "Unit tests",
        PageContext::tests("tests"),
        &page_body(&inventory, &summary),
    )
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
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

    for line in text.lines() {
        let Some((qualified, _)) = line.split_once(": test") else {
            continue;
        };
        let parts: Vec<_> = qualified.split("::").collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts.last().unwrap().to_string();
        let module = parts[..parts.len() - 1].join("::");
        let crate_name = parts[0].to_string();
        entries.push(TestEntry {
            crate_name,
            module,
            name,
        });
    }

    entries.sort_by(|a, b| {
        (&a.crate_name, &a.module, &a.name).cmp(&(&b.crate_name, &b.module, &b.name))
    });
    entries
}

fn run_tests_if_requested() -> TestRunSummary {
    if std::env::var("DOCS_RUN_TESTS").ok().as_deref() != Some("1") {
        return TestRunSummary::default();
    }

    let output = Command::new("cargo")
        .args(["test", "--workspace", "--", "--format", "terse"])
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
        if let Some(rest) = line.strip_prefix("test result:") {
            for part in rest.split(',') {
                let part = part.trim();
                if let Some(n) = part.strip_suffix(" passed") {
                    summary.passed = n.trim().parse().unwrap_or(0);
                } else if let Some(n) = part.strip_suffix(" failed") {
                    summary.failed = n.trim().parse().unwrap_or(0);
                } else if let Some(n) = part.strip_suffix(" ignored") {
                    summary.ignored = n.trim().parse().unwrap_or(0);
                }
            }
        }
    }

    summary
}

fn page_body(inventory: &[TestEntry], summary: &TestRunSummary) -> String {
    let mut status = String::new();
    if summary.ran {
        let ok = summary.failed == 0;
        status.push_str(&format!(
            "<h2>Last run</h2>
<div class=\"meta-grid\">
  <div class=\"meta-item\"><div class=\"meta-label\">Passed</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Failed</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Ignored</div><div class=\"meta-value\">{}</div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Status</div><div class=\"meta-value\">{}</div></div>
</div>",
            summary.passed,
            summary.failed,
            summary.ignored,
            if ok { "All passed" } else { "Failures detected" }
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
