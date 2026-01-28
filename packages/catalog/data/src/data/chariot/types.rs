use kpi_core::{
    kpi::{CategorySummary, KpiMetadata, KpiScore, KpiStatus},
    subject::SubjectSnapshot,
};
use kpi_report::ReportBundle;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowNodeKpiResult {
    pub id: String,
    pub score: u8,
    pub status: KpiStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl FlowNodeKpiResult {
    pub fn from_score(score: &KpiScore) -> Self {
        Self {
            id: score.meta.id.to_string(),
            score: score.score,
            status: score.status,
            notes: score.notes.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowPlainKpiScore {
    pub id: String,
    pub model_area_key: String,
    pub model_area_label: String,
    pub category_label: String,
    pub profit_driver: String,
    pub name: String,
    pub description: String,
    pub group_label: String,
    pub manual_verification_hint: String,
    pub score: u8,
    pub status: KpiStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl From<&KpiScore> for FlowPlainKpiScore {
    fn from(score: &KpiScore) -> Self {
        let meta = score.meta;
        Self {
            id: meta.id.to_string(),
            model_area_key: meta.model_area_key.to_string(),
            model_area_label: meta.model_area_label.to_string(),
            category_label: meta.category_label.to_string(),
            profit_driver: meta.profit_driver.to_string(),
            name: meta.name.to_string(),
            description: meta.description.to_string(),
            group_label: meta.group_label.to_string(),
            manual_verification_hint: meta.manual_verification_hint.to_string(),
            score: score.score,
            status: score.status,
            notes: score.notes.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowReportOutput {
    pub bundle: ReportBundle,
    pub summaries: Vec<CategorySummary>,
    pub scores: Vec<FlowPlainKpiScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowKpiMetadata {
    pub id: String,
    pub model_area_key: String,
    pub model_area_label: String,
    pub category_label: String,
    pub profit_driver: String,
    pub name: String,
    pub description: String,
    pub group_label: String,
    pub manual_verification_hint: String,
}

impl From<&'static KpiMetadata> for FlowKpiMetadata {
    fn from(meta: &'static KpiMetadata) -> Self {
        Self {
            id: meta.id.to_string(),
            model_area_key: meta.model_area_key.to_string(),
            model_area_label: meta.model_area_label.to_string(),
            category_label: meta.category_label.to_string(),
            profit_driver: meta.profit_driver.to_string(),
            name: meta.name.to_string(),
            description: meta.description.to_string(),
            group_label: meta.group_label.to_string(),
            manual_verification_hint: meta.manual_verification_hint.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowReportInput {
    pub snapshot: SubjectSnapshot,
    pub scores: Vec<FlowNodeKpiResult>,
}
