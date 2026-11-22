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
