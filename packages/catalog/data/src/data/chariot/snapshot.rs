use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
};
use flow_like_types::{async_trait, json::{from_value, to_value}, Value as JsonValue};
use kpi_core::subject::{GbpSignals, SubjectSeed, SubjectSnapshot, WebsiteSignals};

#[crate::register_node]
#[derive(Default)]
pub struct MergeSnapshotNode;

#[async_trait]
impl NodeLogic for MergeSnapshotNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_merge_snapshot",
            "Build Subject Snapshot",
            "Combine the SubjectSeed with optional WebsiteSignals and GBP signals into a SubjectSnapshot.",
            "Data/Chariot/Snapshot",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "seed",
            "Subject Seed",
            "SubjectSeed payload produced upstream.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSeed>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "website_signals",
            "Website Signals",
            "Optional WebsiteSignals payload from the crawler.",
            VariableType::Struct,
        )
        .set_schema::<WebsiteSignals>()
        .set_options(PinOptions::new().set_enforce_schema(false).build())
        .set_default_value(Some(JsonValue::Null));

        node.add_input_pin(
            "gbp_signals",
            "GBP Signals",
            "Optional GbpSignals payload from the GBP lookup.",
            VariableType::Struct,
        )
        .set_schema::<GbpSignals>()
        .set_options(PinOptions::new().set_enforce_schema(false).build())
        .set_default_value(Some(JsonValue::Null));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when the snapshot is constructed.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "snapshot",
            "Subject Snapshot",
            "Combined SubjectSnapshot ready for KPI evaluation.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSnapshot>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let seed: SubjectSeed = context.evaluate_pin("seed").await?;
        let website_signals_value: JsonValue = context
            .evaluate_pin("website_signals")
            .await
            .unwrap_or(JsonValue::Null);
        let gbp_signals_value: JsonValue = context
            .evaluate_pin("gbp_signals")
            .await
            .unwrap_or(JsonValue::Null);
        let website_signals = if website_signals_value.is_null() {
            None
        } else {
            Some(from_value(website_signals_value)?)
        };
        let gbp_signals = if gbp_signals_value.is_null() {
            None
        } else {
            Some(from_value(gbp_signals_value)?)
        };

        let snapshot = SubjectSnapshot {
            seed,
            website_signals,
            gbp_signals,
        };

        let value = to_value(&snapshot)?;
        context.set_pin_value("snapshot", value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
