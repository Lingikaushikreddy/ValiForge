use std::fmt::Write;

use valiforge_core::ValidationReport;

pub trait ReportFormatter {
    fn format_id(&self) -> &'static str;
    fn format(&self, report: &ValidationReport) -> String;
}

pub struct JsonFormatter;

impl ReportFormatter for JsonFormatter {
    fn format_id(&self) -> &'static str {
        "json"
    }
    fn format(&self, report: &ValidationReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }
}

pub struct JunitFormatter;

// Writing into a `String` cannot fail, so the `fmt::Result`s below are ignored.
impl ReportFormatter for JunitFormatter {
    fn format_id(&self) -> &'static str {
        "junit"
    }
    fn format(&self, report: &ValidationReport) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(
            xml,
            "<testsuites tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">",
            report.summary.total,
            report.summary.failed,
            report.summary.errors,
            report.duration.as_secs_f64()
        );
        let _ = writeln!(
            xml,
            "  <testsuite name=\"{}\" tests=\"{}\">",
            xml_escape(&report.schema_name),
            report.summary.total
        );
        for result in &report.results {
            let name = xml_escape(&format!("{} {}", result.method, result.path));
            let _ = write!(
                xml,
                "    <testcase name=\"{name}\" time=\"{:.3}\"",
                result.latency.as_secs_f64()
            );
            if result.violations.is_empty() {
                xml.push_str(" />\n");
            } else {
                xml.push_str(">\n");
                for v in &result.violations {
                    let _ = writeln!(
                        xml,
                        "      <failure message=\"{}\" type=\"{}\"/>",
                        xml_escape(&v.message),
                        xml_escape(&v.rule)
                    );
                }
                xml.push_str("    </testcase>\n");
            }
        }
        xml.push_str("  </testsuite>\n</testsuites>\n");
        xml
    }
}

pub struct MarkdownFormatter;

impl ReportFormatter for MarkdownFormatter {
    fn format_id(&self) -> &'static str {
        "markdown"
    }
    fn format(&self, report: &ValidationReport) -> String {
        let mut md = "## ValiForge Validation Report\n\n".to_string();
        let _ = writeln!(
            md,
            "**Schema:** {} | **Target:** {} | **Duration:** {:.1}s\n",
            report.schema_name,
            report.target_url,
            report.duration.as_secs_f64()
        );
        let _ = writeln!(
            md,
            "| Status | Total | Passed | Failed | Errors |\n|---|---|---|---|---|\n| {} | {} | {} | {} | {} |\n",
            if report.is_success() { "PASS" } else { "FAIL" },
            report.summary.total,
            report.summary.passed,
            report.summary.failed,
            report.summary.errors,
        );
        if report.results.iter().all(|r| r.violations.is_empty()) {
            md.push_str("All endpoints passed validation.\n");
        } else {
            md.push_str("### Violations\n\n");
            for r in report.results.iter().filter(|r| !r.violations.is_empty()) {
                let _ = writeln!(md, "**{} {}**", r.method, r.path);
                for v in &r.violations {
                    let _ = writeln!(md, "- `{}`: {}", v.rule, v.message);
                }
                md.push('\n');
            }
        }
        md
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
