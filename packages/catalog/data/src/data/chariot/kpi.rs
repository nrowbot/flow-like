use std::collections::HashMap;

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
    json::{json, to_value},
};
use kpi_core::{
    kpi::{KpiEvaluator, KpiRegistry, KpiScore},
    subject::{SubjectContext, SubjectSnapshot},
};
use kpi_platform_growth::default_registry;

use super::types::{FlowKpiMetadata, FlowNodeKpiResult};

#[crate::register_node]
#[derive(Default)]
pub struct ListChariotKpisNode;

#[async_trait]
impl NodeLogic for ListChariotKpisNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_list_kpis",
            "List KPIs",
            "List metadata describing each KPI that can be evaluated via the Growth registry.",
            "Data/Chariot/KPI",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);
        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered after the registry is loaded.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "kpis",
            "KPI Metadata",
            "Array of KPI metadata entries.",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<FlowKpiMetadata>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let registry = default_registry();
        let metadata: Vec<FlowKpiMetadata> = registry
            .iter()
            .map(|kpi| FlowKpiMetadata::from(kpi.metadata()))
            .collect();
        context.set_pin_value("kpis", json!(metadata)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct EvaluateChariotKpiNode;

#[async_trait]
impl NodeLogic for EvaluateChariotKpiNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_evaluate_kpi",
            "Evaluate KPI",
            "Run a single KPI evaluator against the provided SubjectSnapshot.",
            "Data/Chariot/KPI",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "snapshot",
            "Subject Snapshot",
            "Snapshot built from SubjectSeed + gathered signals.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSnapshot>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "kpi_id",
            "KPI ID",
            "ID of the KPI to evaluate (e.g. digital_presence.https_enabled).",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when the KPI evaluation succeeds.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when evaluation fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "result",
            "KPI Result",
            "Structured KPI score output.",
            VariableType::Struct,
        )
        .set_schema::<FlowNodeKpiResult>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let snapshot: SubjectSnapshot = context.evaluate_pin("snapshot").await?;
        let kpi_id: String = context.evaluate_pin("kpi_id").await?;

        if kpi_id.trim().is_empty() {
            context.log_message("KPI ID cannot be empty.", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let registry = default_registry();
        let Some(kpi) = find_kpi(&registry, &kpi_id) else {
            context.log_message(
                &format!("Unknown KPI id requested: {}", kpi_id),
                LogLevel::Error,
            );
            context.activate_exec_pin("error").await?;
            return Ok(());
        };

        let ctx = SubjectContext {
            snapshot: &snapshot,
        };

        let mut score = kpi.evaluate(&ctx);
        score.meta = kpi.metadata();

        let result = FlowNodeKpiResult::from_score(&score);
        context.set_pin_value("result", to_value(&result)?).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

pub(crate) fn hydrate_scores(
    results: &[FlowNodeKpiResult],
    registry: &KpiRegistry,
) -> flow_like_types::Result<Vec<KpiScore>> {
    let mut lookup: HashMap<&str, &'static dyn KpiEvaluator> = HashMap::new();
    for evaluator in registry {
        lookup.insert(evaluator.metadata().id, *evaluator);
    }

    results
        .iter()
        .map(|result| {
            let evaluator = lookup
                .get(result.id.as_str())
                .ok_or_else(|| flow_like_types::anyhow!("Unknown KPI id: {}", result.id))?;
            Ok(KpiScore {
                meta: evaluator.metadata(),
                score: result.score,
                status: result.status,
                notes: result.notes.clone(),
            })
        })
        .collect()
}

fn find_kpi<'a>(
    registry: &'a [&'static dyn KpiEvaluator],
    id: &str,
) -> Option<&'a &'static dyn KpiEvaluator> {
    registry.iter().find(|kpi| kpi.metadata().id == id)
}
