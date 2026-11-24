//! The loss report: what an import could not carry over, in the shape SolidCraft shows.

use serde::Serialize;

/// The report an importer returns to SolidCraft.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LossReport {
    /// One line on what was imported.
    pub summary: String,
    /// Each thing that was not carried over as it was.
    pub items: Vec<LossItem>,
}

impl LossReport {
    /// A report with this summary and nothing lost.
    #[must_use]
    pub fn new(summary: impl Into<String>) -> Self {
        Self { summary: summary.into(), items: Vec::new() }
    }

    /// Adds an item.
    pub fn push(&mut self, item: LossItem) -> &mut Self {
        self.items.push(item);
        self
    }

    /// The report as the JSON the importer interface returns.
    ///
    /// # Panics
    ///
    /// Never: every field serialises.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a report always serialises")
    }
}

/// How bad a loss is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Worth knowing, nothing is missing.
    Info,
    /// Carried over in a lesser form.
    Degraded,
}

/// One thing that was not carried over as it was.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LossItem {
    /// How bad it is.
    pub severity: Severity,
    /// The kind of loss in the user's words; SolidCraft shows one line per title.
    pub title: String,
    /// What it concerns.
    pub subject: String,
    /// The details.
    pub message: String,
    /// A stable code for this kind of loss.
    pub code: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_serialises_to_the_interface_json() {
        let mut report = LossReport::new("imported as a mesh body");
        report.push(LossItem {
            severity: Severity::Degraded,
            title: "The outline was kept as a mesh".into(),
            subject: "the outline".into(),
            message: "no single radius fits every corner".into(),
            code: "outline_fillet",
        });
        let json: serde_json::Value = serde_json::from_str(&report.to_json()).expect("valid JSON");
        assert_eq!(json["summary"], "imported as a mesh body");
        assert_eq!(json["items"].as_array().expect("items").len(), 1);
        assert_eq!(json["items"][0]["severity"], "degraded");
        assert_eq!(json["items"][0]["title"], "The outline was kept as a mesh");
        assert_eq!(json["items"][0]["code"], "outline_fillet");
    }

    #[test]
    fn an_empty_report_carries_nothing_lost() {
        let report = LossReport::default();
        assert_eq!(report, LossReport::new(""));
        let json: serde_json::Value = serde_json::from_str(&report.to_json()).expect("valid JSON");
        assert_eq!(json["items"], serde_json::json!([]));
        assert_eq!(serde_json::to_string(&Severity::Info).expect("serialises"), "\"info\"");
    }
}
