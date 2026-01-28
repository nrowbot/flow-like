use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
};
use flow_like_types::{
    async_trait,
    json::to_value,
};
use kpi_core::{kpi::summarize_by_area, subject::SubjectSnapshot};
use kpi_platform_growth::default_registry;
use kpi_report::build_report_bundle;

use super::{
    kpi::hydrate_scores,
    types::{FlowNodeKpiResult, FlowPlainKpiScore, FlowReportOutput},
};

#[crate::register_node]
#[derive(Default)]
pub struct BuildChariotReportNode;

#[async_trait]
impl NodeLogic for BuildChariotReportNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_build_report",
            "Build Report Bundle",
            "Convert KPI scores and a SubjectSnapshot into the Growth report bundle + summaries.",
            "Data/Chariot/Report",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "snapshot",
            "Subject Snapshot",
            "Snapshot describing the subject that was scored.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSnapshot>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "scores",
            "KPI Results",
            "Array of KPI results emitted by the Evaluate KPI node.",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<FlowNodeKpiResult>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when the report is generated.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when bundling fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "report",
            "Report Bundle",
            "Structured output including ReportBundle + summaries.",
            VariableType::Struct,
        )
        .set_schema::<FlowReportOutput>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let snapshot: SubjectSnapshot = context.evaluate_pin("snapshot").await?;
        let scores: Vec<FlowNodeKpiResult> = context.evaluate_pin("scores").await?;

        let registry = default_registry();
        let hydrated = match hydrate_scores(&scores, &registry) {
            Ok(values) => values,
            Err(err) => {
                context.log_message(
                    &format!("Failed to hydrate KPI scores: {err}"),
                    LogLevel::Error,
                );
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let summaries = summarize_by_area(&hydrated);
        let bundle = build_report_bundle(&snapshot, &hydrated, &summaries);
        let plain_scores: Vec<FlowPlainKpiScore> =
            hydrated.iter().map(FlowPlainKpiScore::from).collect();

        let output = FlowReportOutput {
            bundle,
            summaries,
            scores: plain_scores,
        };

        context.set_pin_value("report", to_value(&output)?).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
