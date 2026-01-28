use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
};
use flow_like_types::{
    Value, async_trait,
    json::{json, to_value},
    tokio::task,
};
use kpi_cli::{gbp, signals};
use kpi_core::subject::{GbpSignals, SubjectSeed, WebsiteSignals};

#[crate::register_node]
#[derive(Default)]
pub struct GatherWebsiteSignalsNode;

#[async_trait]
impl NodeLogic for GatherWebsiteSignalsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_gather_website_signals",
            "Gather Website Signals",
            "Crawl a subject website and emit structured WebsiteSignals used by Chariot KPIs.",
            "Data/Chariot/Signals",
        );
        node.add_icon("/flow/icons/web.svg");
        node.set_long_running(true);

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "seed",
            "Subject Seed",
            "SubjectSeed payload that includes the website URL.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSeed>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "debug",
            "Debug Logging",
            "Enable to print verbose crawl output to STDOUT/STDERR.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "disable_browser_solve",
            "Disable Browser Solve",
            "Disable browser-based SiteGround/captcha solve when blocked.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "max_additional_pages",
            "Max Additional Pages",
            "Cap the number of extra pages crawled beyond the homepage.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "max_sitemap_candidates",
            "Max Sitemap Candidates",
            "Cap the number of sitemap URLs parsed for feature pages.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "request_timeout_secs",
            "Request Timeout (secs)",
            "HTTP timeout for crawl requests.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(15)));

        node.add_input_pin(
            "skip_sitemap",
            "Skip Sitemaps",
            "Skip sitemap discovery to speed up crawls.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when the crawl completes successfully.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when the crawl fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "website_signals",
            "Website Signals",
            "Structured WebsiteSignals describing the subject domain.",
            VariableType::Struct,
        )
        .set_schema::<WebsiteSignals>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let seed: SubjectSeed = context.evaluate_pin("seed").await?;
        let debug: bool = context.evaluate_pin("debug").await.unwrap_or(false);
        let max_sitemap_candidates: i64 = context
            .evaluate_pin("max_sitemap_candidates")
            .await
            .unwrap_or(10);
        let max_additional_pages: i64 = context
            .evaluate_pin("max_additional_pages")
            .await
            .unwrap_or(10);
        let request_timeout_secs: i64 = context
            .evaluate_pin("request_timeout_secs")
            .await
            .unwrap_or(15);
        let disable_browser_solve: bool = context
            .evaluate_pin("disable_browser_solve")
            .await
            .unwrap_or(false);
        let skip_sitemap: bool = context.evaluate_pin("skip_sitemap").await.unwrap_or(false);

        let options = signals::WebsiteSignalOptions {
            max_additional_pages: max_additional_pages.max(0) as usize,
            max_sitemap_candidates: max_sitemap_candidates.max(0) as usize,
            request_timeout_secs: request_timeout_secs.max(1) as u64,
            skip_sitemap,
            allow_browser_solve: !disable_browser_solve,
        };

        let Some(url) = seed
            .website
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.to_string())
        else {
            context.log_message(
                "SubjectSeed is missing a website URL to crawl.",
                LogLevel::Error,
            );
            context.activate_exec_pin("error").await?;
            return Ok(());
        };

        let website_signals =
            match signals::gather_website_signals_with_options(url.as_str(), debug, &options).await
            {
                Ok(data) => data,
                Err(err) => {
                    context.log_message(
                        &format!("Failed to gather website signals: {err}"),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }
            };

        let value: Value = to_value(&website_signals)?;
        context.set_pin_value("website_signals", value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}



#[crate::register_node]
#[derive(Default)]
pub struct DiscoverWebsiteCandidatesNode;

#[async_trait]
impl NodeLogic for DiscoverWebsiteCandidatesNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_discover_website_candidates",
            "Discover Website Candidates",
            "Discover candidate URLs and homepage HTML for Chariot website signals.",
            "Data/Chariot/Signals",
        );
        node.add_icon("/flow/icons/web.svg");
        node.set_long_running(true);

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "seed",
            "Subject Seed",
            "SubjectSeed payload that includes the website URL.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSeed>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "debug",
            "Debug Logging",
            "Enable to print verbose crawl output to STDOUT/STDERR.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "max_sitemap_candidates",
            "Max Sitemap Candidates",
            "Cap the number of sitemap URLs parsed for feature pages.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "max_additional_pages",
            "Max Additional Pages",
            "Cap the number of candidate URLs returned for additional pages.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "request_timeout_secs",
            "Request Timeout (secs)",
            "HTTP timeout for crawl requests.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(15)));

        node.add_input_pin(
            "disable_browser_solve",
            "Disable Browser Solve",
            "Disable browser-based SiteGround/captcha solve when blocked.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "skip_sitemap",
            "Skip Sitemaps",
            "Skip sitemap discovery to speed up crawls.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when discovery completes successfully.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when discovery fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "normalized_url",
            "Normalized URL",
            "Normalized root URL for the subject website.",
            VariableType::String,
        );

        node.add_output_pin(
            "homepage_html",
            "Homepage HTML",
            "HTML captured from the homepage (empty when unavailable).",
            VariableType::String,
        );

        node.add_output_pin(
            "candidate_urls",
            "Candidate URLs",
            "Discovered candidate URLs to crawl.",
            VariableType::String,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "robots_txt_present",
            "Robots.txt Present",
            "Whether robots.txt was detected (null when unknown).",
            VariableType::Struct,
        )
        .set_schema::<Option<bool>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "sitemap_present",
            "Sitemap Present",
            "Whether a sitemap was detected (null when unknown).",
            VariableType::Struct,
        )
        .set_schema::<Option<bool>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let seed: SubjectSeed = context.evaluate_pin("seed").await?;
        let debug: bool = context.evaluate_pin("debug").await.unwrap_or(false);
        let max_sitemap_candidates: i64 = context
            .evaluate_pin("max_sitemap_candidates")
            .await
            .unwrap_or(10);
        let max_additional_pages: i64 = context
            .evaluate_pin("max_additional_pages")
            .await
            .unwrap_or(10);
        let request_timeout_secs: i64 = context
            .evaluate_pin("request_timeout_secs")
            .await
            .unwrap_or(15);
        let disable_browser_solve: bool = context
            .evaluate_pin("disable_browser_solve")
            .await
            .unwrap_or(false);
        let skip_sitemap: bool = context.evaluate_pin("skip_sitemap").await.unwrap_or(false);

        let options = signals::WebsiteSignalOptions {
            max_additional_pages: max_additional_pages.max(0) as usize,
            max_sitemap_candidates: max_sitemap_candidates.max(0) as usize,
            request_timeout_secs: request_timeout_secs.max(1) as u64,
            skip_sitemap,
            allow_browser_solve: !disable_browser_solve,
        };

        let Some(url) = seed
            .website
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.to_string())
        else {
            context.log_message(
                "SubjectSeed is missing a website URL to crawl.",
                LogLevel::Error,
            );
            context.activate_exec_pin("error").await?;
            return Ok(());
        };

        let discovery = match signals::discover_website_candidates(url.as_str(), debug, &options)
            .await
        {
            Ok(value) => value,
            Err(err) => {
                context.log_message(
                    &format!("Failed to discover website candidates: {err}"),
                    LogLevel::Error,
                );
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let mut candidates = discovery.candidates;
        let candidate_limit = max_additional_pages.max(0) as usize;
        if candidate_limit > 0 && candidates.len() > candidate_limit {
            candidates.truncate(candidate_limit);
        }

        context
            .set_pin_value("normalized_url", json!(discovery.normalized_url))
            .await?;
        context
            .set_pin_value("homepage_html", json!(discovery.homepage_html.unwrap_or_default()))
            .await?;
        context
            .set_pin_value("candidate_urls", json!(candidates))
            .await?;
        context
            .set_pin_value("robots_txt_present", json!(discovery.robots_txt_present))
            .await?;
        context
            .set_pin_value("sitemap_present", json!(discovery.sitemap_present))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct FetchWebsitePageNode;

#[async_trait]
impl NodeLogic for FetchWebsitePageNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_fetch_website_page",
            "Fetch Website Page",
            "Fetch a single website page HTML for Chariot parsing.",
            "Data/Chariot/Signals",
        );
        node.add_icon("/flow/icons/web.svg");
        node.set_long_running(true);

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "url",
            "URL",
            "Full page URL to fetch.",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "debug",
            "Debug Logging",
            "Enable to print verbose crawl output to STDOUT/STDERR.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "request_timeout_secs",
            "Request Timeout (secs)",
            "HTTP timeout for crawl requests.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(15)));

        node.add_input_pin(
            "disable_browser_solve",
            "Disable Browser Solve",
            "Disable browser-based SiteGround/captcha solve when blocked.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when the fetch completes.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when the fetch fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "page_html",
            "Page HTML",
            "HTML payload for the requested page (empty when unavailable).",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let url: String = context.evaluate_pin("url").await.unwrap_or_default();
        let debug: bool = context.evaluate_pin("debug").await.unwrap_or(false);
        let request_timeout_secs: i64 = context
            .evaluate_pin("request_timeout_secs")
            .await
            .unwrap_or(15);
        let disable_browser_solve: bool = context
            .evaluate_pin("disable_browser_solve")
            .await
            .unwrap_or(false);

        if url.trim().is_empty() {
            context.log_message("URL is required.", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let html = match signals::fetch_website_page_html(
            url.as_str(),
            debug,
            request_timeout_secs.max(1) as u64,
            !disable_browser_solve,
        )
        .await
        {
            Ok(Some(body)) => body,
            Ok(None) => {
                context.log_message(
                    &format!("No HTML returned for {url}"),
                    LogLevel::Warn,
                );
                String::new()
            }
            Err(err) => {
                context.log_message(&format!("Failed to fetch {url}: {err}"), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        context.set_pin_value("page_html", json!(html)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ParseWebsiteSignalsNode;

#[async_trait]
impl NodeLogic for ParseWebsiteSignalsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_parse_website_signals",
            "Parse Website Signals",
            "Parse HTML pages into WebsiteSignals for Chariot.",
            "Data/Chariot/Signals",
        );
        node.add_icon("/flow/icons/web.svg");
        node.set_long_running(true);

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "debug",
            "Debug Logging",
            "Enable verbose parsing timing logs.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "normalized_url",
            "Normalized URL",
            "Normalized root URL for the subject website.",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "homepage_html",
            "Homepage HTML",
            "HTML captured from the homepage (empty when unavailable).",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "extra_pages_html",
            "Extra Pages HTML",
            "HTML payloads for additional pages.",
            VariableType::String,
        )
        .set_value_type(ValueType::Array)
        .set_default_value(Some(json!([])));

        node.add_input_pin(
            "robots_txt_present",
            "Robots.txt Present",
            "Whether robots.txt was detected (null when unknown).",
            VariableType::Struct,
        )
        .set_schema::<Option<bool>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "sitemap_present",
            "Sitemap Present",
            "Whether a sitemap was detected (null when unknown).",
            VariableType::Struct,
        )
        .set_schema::<Option<bool>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when parsing completes.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when parsing fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "website_signals",
            "Website Signals",
            "Structured WebsiteSignals describing the subject domain.",
            VariableType::Struct,
        )
        .set_schema::<WebsiteSignals>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let normalized_url: String = context.evaluate_pin("normalized_url").await.unwrap_or_default();
        let debug: bool = context.evaluate_pin("debug").await.unwrap_or(false);
        let homepage_html: String = context.evaluate_pin("homepage_html").await.unwrap_or_default();
        let extra_pages_html: Vec<String> = context
            .evaluate_pin("extra_pages_html")
            .await
            .unwrap_or_default();
        let robots_txt_present: Option<bool> = context
            .evaluate_pin("robots_txt_present")
            .await
            .unwrap_or(None);
        let sitemap_present: Option<bool> = context
            .evaluate_pin("sitemap_present")
            .await
            .unwrap_or(None);

        if normalized_url.trim().is_empty() {
            context.log_message("Normalized URL is required.", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let homepage_html = if homepage_html.trim().is_empty() {
            None
        } else {
            Some(homepage_html)
        };

        let signals = match signals::parse_website_signals_from_pages(
            normalized_url.as_str(),
            homepage_html,
            &extra_pages_html,
            robots_txt_present,
            sitemap_present,
            debug,
        ) {
            Ok(value) => value,
            Err(err) => {
                context.log_message(&format!("Failed to parse website signals: {err}"), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let value: Value = to_value(&signals)?;
        context.set_pin_value("website_signals", value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct GatherGbpSignalsNode;

#[async_trait]
impl NodeLogic for GatherGbpSignalsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "chariot_gather_gbp_signals",
            "Gather GBP Signals",
            "Fetch Google Business Profile (GBP) insights for a SubjectSeed.",
            "Data/Chariot/Signals",
        );
        node.add_icon("/flow/icons/web.svg");
        node.set_long_running(true);

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin(
            "seed",
            "Subject Seed",
            "SubjectSeed payload that informs the GBP lookup.",
            VariableType::Struct,
        )
        .set_schema::<SubjectSeed>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "debug",
            "Debug Logging",
            "Enable to print verbose Places API responses.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when GBP signals are fetched.",
            VariableType::Execution,
        );
        node.add_output_pin(
            "error",
            "Error",
            "Triggered when the GBP lookup fails.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "gbp_signals",
            "GBP Signals",
            "Optional GBP signal payload; Null indicates no listing was found or API disabled.",
            VariableType::Struct,
        )
        .set_schema::<Option<GbpSignals>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let seed: SubjectSeed = context.evaluate_pin("seed").await?;
        let debug: bool = context.evaluate_pin("debug").await.unwrap_or(false);

        let signals_opt = match gbp::gather_gbp_signals(&seed, debug).await {
            Ok(result) => result,
            Err(err) => {
                context.log_message(
                    &format!("Failed to gather GBP signals: {err}"),
                    LogLevel::Error,
                );
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let value = json!(signals_opt);
        context.set_pin_value("gbp_signals", value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
