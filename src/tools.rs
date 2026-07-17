//! Curated tool catalog and argument dispatch.
//!
//! Instead of auto-generating one tool per Matomo API method (70+ tools that
//! flood the model's context), we expose ~14 hand-crafted tools that cover the
//! common analytics questions, plus `matomo_api` as an escape hatch for the
//! full Reporting API.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Specs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
    String,
    Integer,
    Boolean,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Requirement {
    Required,
    Optional,
    /// Required only when no default site is configured.
    SiteId,
}

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: &'static str,
    /// Matomo query parameter name, when it differs from `name`.
    pub matomo_name: Option<&'static str>,
    pub description: &'static str,
    pub kind: ParamKind,
    pub requirement: Requirement,
    pub default: Option<&'static str>,
    pub choices: &'static [&'static str],
}

impl ParamSpec {
    const fn new(name: &'static str, kind: ParamKind, description: &'static str) -> Self {
        Self {
            name,
            matomo_name: None,
            description,
            kind,
            requirement: Requirement::Optional,
            default: None,
            choices: &[],
        }
    }

    const fn required(mut self) -> Self {
        self.requirement = Requirement::Required;
        self
    }

    const fn matomo(mut self, name: &'static str) -> Self {
        self.matomo_name = Some(name);
        self
    }

    const fn default_value(mut self, value: &'static str) -> Self {
        self.default = Some(value);
        self
    }

    const fn choices(mut self, values: &'static [&'static str]) -> Self {
        self.choices = values;
        self
    }

    fn matomo_key(&self) -> &'static str {
        self.matomo_name.unwrap_or(self.name)
    }
}

/// One selectable report inside a tool (value of the tool's select argument).
#[derive(Debug, Clone)]
pub struct SelectCase {
    pub value: &'static str,
    pub method: &'static str,
    pub fixed: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone)]
pub enum Binding {
    /// One fixed Matomo method.
    Fixed {
        method: &'static str,
        fixed: &'static [(&'static str, &'static str)],
    },
    /// Method chosen by the value of `arg` (first case is the default).
    Select {
        arg: &'static str,
        cases: &'static [SelectCase],
    },
    /// Generic escape hatch: expects `method` and `params` arguments.
    Raw,
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub params: Vec<ParamSpec>,
    pub binding: Binding,
}

/// A fully resolved Matomo API call.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub method: String,
    pub params: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Common parameters
// ---------------------------------------------------------------------------

const fn p_site() -> ParamSpec {
    ParamSpec {
        name: "site_id",
        matomo_name: Some("idSite"),
        description: "Numeric Matomo site ID. If unknown, call matomo_list_sites first.",
        kind: ParamKind::Integer,
        requirement: Requirement::SiteId,
        default: None,
        choices: &[],
    }
}

const fn p_period() -> ParamSpec {
    ParamSpec::new(
        "period",
        ParamKind::String,
        "Aggregation period for the report.",
    )
    .choices(&["day", "week", "month", "year", "range"])
    .default_value("day")
}

const fn p_date() -> ParamSpec {
    ParamSpec::new(
        "date",
        ParamKind::String,
        "Date for the report: 'today', 'yesterday', 'YYYY-MM-DD', a rolling window like \
         'last7' or 'last30', or a range 'YYYY-MM-DD,YYYY-MM-DD' (combine with period=range).",
    )
    .default_value("yesterday")
}

const fn p_segment() -> ParamSpec {
    ParamSpec::new(
        "segment",
        ParamKind::String,
        "Optional Matomo segment filter, e.g. 'deviceType==mobile;country==DE'. \
         See https://matomo.org/docs/segmentation/ for the syntax.",
    )
}

const fn p_limit(default: &'static str) -> ParamSpec {
    ParamSpec::new(
        "limit",
        ParamKind::Integer,
        "Maximum number of rows to return (-1 for all rows).",
    )
    .matomo("filter_limit")
    .default_value(default)
}

fn report_params(limit: Option<&'static str>) -> Vec<ParamSpec> {
    let mut params = vec![p_site(), p_period(), p_date(), p_segment()];
    if let Some(default) = limit {
        params.push(p_limit(default));
    }
    params
}

fn select_param(
    name: &'static str,
    description: &'static str,
    cases: &'static [SelectCase],
) -> ParamSpec {
    // Choices are validated against the binding's cases in `resolve`; the enum
    // shown in the schema is derived there too.
    let mut p = ParamSpec::new(name, ParamKind::String, description);
    p.default = Some(cases[0].value);
    p
}

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

macro_rules! cases {
    ($($value:literal => $method:literal $([$(($fk:literal, $fv:literal)),+])?),+ $(,)?) => {
        &[$(SelectCase {
            value: $value,
            method: $method,
            fixed: &[$($(($fk, $fv)),+)?],
        }),+]
    };
}

pub fn catalog() -> Vec<ToolSpec> {
    const PAGES_CASES: &[SelectCase] = cases! {
        "page_urls" => "Actions.getPageUrls" [("flat", "1")],
        "page_titles" => "Actions.getPageTitles" [("flat", "1")],
        "entry_pages" => "Actions.getEntryPageUrls" [("flat", "1")],
        "exit_pages" => "Actions.getExitPageUrls" [("flat", "1")],
        "downloads" => "Actions.getDownloads" [("flat", "1")],
        "outlinks" => "Actions.getOutlinks" [("flat", "1")],
    };
    const REFERRER_CASES: &[SelectCase] = cases! {
        "overview" => "Referrers.get",
        "channel_types" => "Referrers.getReferrerType",
        "all" => "Referrers.getAll",
        "websites" => "Referrers.getWebsites",
        "search_engines" => "Referrers.getSearchEngines",
        "keywords" => "Referrers.getKeywords",
        "social_networks" => "Referrers.getSocials",
        "campaigns" => "Referrers.getCampaigns",
        "ai_assistants" => "Referrers.getAIAssistants",
    };
    const EVENT_CASES: &[SelectCase] = cases! {
        "categories" => "Events.getCategory",
        "actions" => "Events.getAction",
        "names" => "Events.getName",
    };
    const GOAL_CASES: &[SelectCase] = cases! {
        "conversions" => "Goals.get",
        "list" => "Goals.getGoals",
    };
    const ECOMMERCE_CASES: &[SelectCase] = cases! {
        "overview" => "Goals.get" [("idGoal", "ecommerceOrder")],
        "products" => "Goals.getItemsName",
        "skus" => "Goals.getItemsSku",
        "categories" => "Goals.getItemsCategory",
    };
    const GEO_CASES: &[SelectCase] = cases! {
        "country" => "UserCountry.getCountry",
        "continent" => "UserCountry.getContinent",
        "region" => "UserCountry.getRegion",
        "city" => "UserCountry.getCity",
    };
    const DEVICE_CASES: &[SelectCase] = cases! {
        "device_type" => "DevicesDetection.getType",
        "brand" => "DevicesDetection.getBrand",
        "model" => "DevicesDetection.getModel",
        "browser" => "DevicesDetection.getBrowsers",
        "browser_version" => "DevicesDetection.getBrowserVersions",
        "os" => "DevicesDetection.getOsFamilies",
        "os_version" => "DevicesDetection.getOsVersions",
        "resolution" => "Resolution.getResolution",
    };
    const VISIT_TIME_CASES: &[SelectCase] = cases! {
        "day_of_week" => "VisitTime.getByDayOfWeek",
        "server_hour" => "VisitTime.getVisitInformationPerServerTime",
        "local_hour" => "VisitTime.getVisitInformationPerLocalTime",
    };
    const SITE_SEARCH_CASES: &[SelectCase] = cases! {
        "keywords" => "Actions.getSiteSearchKeywords",
        "no_result_keywords" => "Actions.getSiteSearchNoResultKeywords",
        "categories" => "Actions.getSiteSearchCategories",
    };
    const REALTIME_CASES: &[SelectCase] = cases! {
        "counters" => "Live.getCounters",
        "last_visits" => "Live.getLastVisitsDetails",
    };

    vec![
        ToolSpec {
            name: "matomo_list_sites",
            description: "List all websites in Matomo that this token can access, with their ID, \
                          name and main URL. Call this first whenever the site_id is unknown.",
            params: vec![p_limit("100")],
            binding: Binding::Fixed {
                method: "SitesManager.getSitesWithAtLeastViewAccess",
                fixed: &[],
            },
        },
        ToolSpec {
            name: "matomo_visits_summary",
            description: "Key traffic metrics for a site and period: visits, unique visitors, \
                          actions (pageviews), bounce rate, actions per visit, and average visit \
                          duration. The go-to tool for 'how much traffic did we get?'.",
            params: report_params(None),
            binding: Binding::Fixed {
                method: "VisitsSummary.get",
                fixed: &[],
            },
        },
        ToolSpec {
            name: "matomo_pages",
            description: "Page-level analytics: most visited page URLs or titles, entry and exit \
                          pages, file downloads, and clicked outbound links. URLs are returned \
                          flattened (full paths), sorted by visits.",
            params: {
                let mut p = vec![select_param(
                    "report",
                    "Which page report to fetch.",
                    PAGES_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "report",
                cases: PAGES_CASES,
            },
        },
        ToolSpec {
            name: "matomo_referrers",
            description: "Where traffic comes from: channel overview (direct, search, websites, \
                          social, campaigns), referring websites, search engines and keywords, \
                          social networks, campaign performance, and AI assistants (Matomo 5.1+).",
            params: {
                let mut p = vec![select_param(
                    "report",
                    "Which referrer report to fetch.",
                    REFERRER_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "report",
                cases: REFERRER_CASES,
            },
        },
        ToolSpec {
            name: "matomo_events",
            description: "Custom event tracking reports (clicks, video plays, form interactions, \
                          ...), grouped by event category, action, or name.",
            params: {
                let mut p = vec![select_param(
                    "group_by",
                    "How to group the tracked events.",
                    EVENT_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "group_by",
                cases: EVENT_CASES,
            },
        },
        ToolSpec {
            name: "matomo_goals",
            description: "Goal conversions: overall conversion counts, rates and revenue \
                          (report=conversions), or the list of configured goals with their IDs \
                          (report=list).",
            params: {
                let mut p = vec![select_param(
                    "report",
                    "Conversion metrics or the list of configured goals.",
                    GOAL_CASES,
                )];
                p.push(
                    ParamSpec::new(
                        "goal_id",
                        ParamKind::Integer,
                        "Optional numeric goal ID to restrict conversion metrics to one goal \
                         (find IDs via report=list).",
                    )
                    .matomo("idGoal"),
                );
                p.extend(report_params(None));
                p
            },
            binding: Binding::Select {
                arg: "report",
                cases: GOAL_CASES,
            },
        },
        ToolSpec {
            name: "matomo_ecommerce",
            description: "E-commerce performance: revenue/order overview, and best-selling \
                          products by product name, SKU, or category.",
            params: {
                let mut p = vec![select_param(
                    "report",
                    "Which e-commerce report to fetch.",
                    ECOMMERCE_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "report",
                cases: ECOMMERCE_CASES,
            },
        },
        ToolSpec {
            name: "matomo_geo",
            description: "Visitor locations: visits broken down by country, continent, region, \
                          or city.",
            params: {
                let mut p = vec![select_param("level", "Geographic granularity.", GEO_CASES)];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "level",
                cases: GEO_CASES,
            },
        },
        ToolSpec {
            name: "matomo_devices",
            description: "Devices and technology used by visitors: device types (desktop, mobile, \
                          tablet), brands, models, browsers, browser versions, operating systems, \
                          and screen resolutions.",
            params: {
                let mut p = vec![select_param(
                    "dimension",
                    "Which device/technology dimension to report on.",
                    DEVICE_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "dimension",
                cases: DEVICE_CASES,
            },
        },
        ToolSpec {
            name: "matomo_visit_times",
            description: "When visitors come to the site: traffic by day of week, or by hour of \
                          day (server time or the visitor's local time).",
            params: {
                let mut p = vec![select_param(
                    "dimension",
                    "Time dimension for the breakdown.",
                    VISIT_TIME_CASES,
                )];
                p.extend(report_params(None));
                p
            },
            binding: Binding::Select {
                arg: "dimension",
                cases: VISIT_TIME_CASES,
            },
        },
        ToolSpec {
            name: "matomo_site_search",
            description: "Internal site-search analytics: what visitors searched for on the site, \
                          searches that returned no results, and search categories.",
            params: {
                let mut p = vec![select_param(
                    "report",
                    "Which site-search report to fetch.",
                    SITE_SEARCH_CASES,
                )];
                p.extend(report_params(Some("20")));
                p
            },
            binding: Binding::Select {
                arg: "report",
                cases: SITE_SEARCH_CASES,
            },
        },
        ToolSpec {
            name: "matomo_realtime",
            description: "Real-time analytics: live visitor/action/conversion counters for the \
                          last N minutes (report=counters), or a detailed log of the most recent \
                          individual visits (report=last_visits).",
            params: vec![
                select_param(
                    "report",
                    "Live counters or recent visit details.",
                    REALTIME_CASES,
                ),
                p_site(),
                ParamSpec::new(
                    "last_minutes",
                    ParamKind::Integer,
                    "Time window in minutes for report=counters.",
                )
                .matomo("lastMinutes")
                .default_value("30"),
                p_limit("10"),
                p_segment(),
            ],
            binding: Binding::Select {
                arg: "report",
                cases: REALTIME_CASES,
            },
        },
        ToolSpec {
            name: "matomo_page_performance",
            description: "Page load performance: average network, server, transfer, DOM \
                          processing and rendering times across pageviews.",
            params: report_params(None),
            binding: Binding::Fixed {
                method: "PagePerformance.get",
                fixed: &[],
            },
        },
        ToolSpec {
            name: "matomo_api",
            description: "Escape hatch: call ANY Matomo Reporting API method directly. Prefer the \
                          dedicated matomo_* tools; use this for reports they don't cover (custom \
                          dimensions, funnels, heatmaps, segment management, ...). Discover \
                          available methods with method='API.getReportMetadata'.",
            params: vec![
                ParamSpec::new(
                    "method",
                    ParamKind::String,
                    "API method as 'Module.action', e.g. 'VisitFrequency.get' or \
                     'API.getReportMetadata'.",
                )
                .required(),
                ParamSpec::new(
                    "params",
                    ParamKind::Object,
                    "Query parameters using Matomo's native names, e.g. \
                     {\"idSite\": 1, \"period\": \"day\", \"date\": \"yesterday\", \
                     \"filter_limit\": 20}.",
                ),
            ],
            binding: Binding::Raw,
        },
    ]
}

// ---------------------------------------------------------------------------
// Registry: schema building + dispatch
// ---------------------------------------------------------------------------

pub struct Registry {
    specs: Vec<ToolSpec>,
    index: HashMap<&'static str, usize>,
    mcp_tools: Vec<Tool>,
    default_site_id: Option<u64>,
}

impl Registry {
    pub fn new(default_site_id: Option<u64>) -> Self {
        let specs = catalog();
        let index = specs.iter().enumerate().map(|(i, s)| (s.name, i)).collect();
        let mcp_tools = specs
            .iter()
            .map(|s| build_mcp_tool(s, default_site_id))
            .collect();
        Self {
            specs,
            index,
            mcp_tools,
            default_site_id,
        }
    }

    pub fn mcp_tools(&self) -> Vec<Tool> {
        self.mcp_tools.clone()
    }

    pub fn tool_count(&self) -> usize {
        self.specs.len()
    }

    /// Resolve a tool call into a concrete Matomo API invocation.
    pub fn resolve(
        &self,
        tool_name: &str,
        args: &Map<String, Value>,
    ) -> Result<Invocation, String> {
        let spec = self
            .index
            .get(tool_name)
            .map(|&i| &self.specs[i])
            .ok_or_else(|| format!("unknown tool '{tool_name}'"))?;

        match &spec.binding {
            Binding::Raw => resolve_raw(args),
            Binding::Fixed { method, fixed } => {
                let params = self.collect_params(spec, args, None)?;
                Ok(Invocation {
                    method: (*method).to_string(),
                    params: apply_fixed(params, fixed),
                })
            }
            Binding::Select { arg, cases } => {
                let requested = match args.get(*arg) {
                    Some(Value::String(s)) => s.as_str(),
                    Some(other) => {
                        return Err(format!("argument '{arg}' must be a string, got {other}"))
                    }
                    None => cases[0].value,
                };
                let case = cases.iter().find(|c| c.value == requested).ok_or_else(|| {
                    format!(
                        "invalid value '{requested}' for '{arg}'. Valid values: {}",
                        cases.iter().map(|c| c.value).collect::<Vec<_>>().join(", ")
                    )
                })?;
                let params = self.collect_params(spec, args, Some(*arg))?;
                Ok(Invocation {
                    method: case.method.to_string(),
                    params: apply_fixed(params, case.fixed),
                })
            }
        }
    }

    fn collect_params(
        &self,
        spec: &ToolSpec,
        args: &Map<String, Value>,
        skip: Option<&str>,
    ) -> Result<Vec<(String, String)>, String> {
        let mut out = Vec::new();

        for param in &spec.params {
            if skip == Some(param.name) {
                continue;
            }

            let value = match args.get(param.name) {
                Some(Value::Null) | None => None,
                Some(v) => Some(stringify(param, v)?),
            };

            match (value, param.requirement) {
                (Some(v), _) => out.push((param.matomo_key().to_string(), v)),
                (None, Requirement::SiteId) => match self.default_site_id {
                    Some(id) => out.push(("idSite".to_string(), id.to_string())),
                    None => {
                        return Err("site_id is required (no default site configured). \
                             Call matomo_list_sites to find the right site ID, or start the \
                             server with --default-site-id."
                            .to_string())
                    }
                },
                (None, Requirement::Required) => {
                    return Err(format!("required argument '{}' is missing", param.name))
                }
                (None, Requirement::Optional) => {
                    if let Some(default) = param.default {
                        out.push((param.matomo_key().to_string(), default.to_string()));
                    }
                }
            }
        }

        Ok(out)
    }
}

/// Fixed parameters always win over user-supplied ones.
fn apply_fixed(
    mut params: Vec<(String, String)>,
    fixed: &[(&'static str, &'static str)],
) -> Vec<(String, String)> {
    for (k, v) in fixed {
        params.retain(|(name, _)| name != k);
        params.push(((*k).to_string(), (*v).to_string()));
    }
    params
}

/// Parameters the model must never override on the raw tool.
const RESERVED: &[&str] = &["module", "method", "format", "token_auth"];

fn resolve_raw(args: &Map<String, Value>) -> Result<Invocation, String> {
    let method = args
        .get("method")
        .and_then(Value::as_str)
        .ok_or("required argument 'method' is missing")?
        .trim();

    let valid = method.split_once('.').is_some_and(|(module, action)| {
        let ok =
            |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        ok(module) && ok(action) && !action.contains('.')
    });
    if !valid {
        return Err(format!(
            "invalid method '{method}': expected 'Module.action', e.g. 'VisitsSummary.get'"
        ));
    }

    let mut params = Vec::new();
    if let Some(raw) = args.get("params") {
        let obj = raw
            .as_object()
            .ok_or("argument 'params' must be a JSON object")?;
        for (key, value) in obj {
            if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(key)) {
                continue;
            }
            let text = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                Value::Null => continue,
                other => other.to_string(),
            };
            params.push((key.clone(), text));
        }
    }

    Ok(Invocation {
        method: method.to_string(),
        params,
    })
}

fn stringify(param: &ParamSpec, value: &Value) -> Result<String, String> {
    let fail = |expected: &str| {
        Err(format!(
            "argument '{}' must be {expected}, got: {value}",
            param.name
        ))
    };

    let text = match (param.kind, value) {
        (ParamKind::Integer, Value::Number(n)) if n.is_i64() || n.is_u64() => n.to_string(),
        (ParamKind::Integer, Value::String(s)) if s.parse::<i64>().is_ok() => s.clone(),
        (ParamKind::Integer, _) => return fail("an integer"),

        (ParamKind::Boolean, Value::Bool(b)) => if *b { "1" } else { "0" }.to_string(),
        (ParamKind::Boolean, Value::String(s)) => match s.as_str() {
            "true" | "1" => "1".to_string(),
            "false" | "0" => "0".to_string(),
            _ => return fail("a boolean"),
        },
        (ParamKind::Boolean, _) => return fail("a boolean"),

        (ParamKind::String, Value::String(s)) => s.clone(),
        (ParamKind::String, Value::Number(n)) => n.to_string(),
        (ParamKind::String, _) => return fail("a string"),

        (ParamKind::Object, v) => v.to_string(),
    };

    if !param.choices.is_empty() && !param.choices.contains(&text.as_str()) {
        return Err(format!(
            "invalid value '{text}' for '{}'. Valid values: {}",
            param.name,
            param.choices.join(", ")
        ));
    }

    Ok(text)
}

fn build_mcp_tool(spec: &ToolSpec, default_site_id: Option<u64>) -> Tool {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for param in &spec.params {
        let mut prop = Map::new();

        let json_type = match param.kind {
            ParamKind::String => "string",
            ParamKind::Integer => "integer",
            ParamKind::Boolean => "boolean",
            ParamKind::Object => "object",
        };
        prop.insert("type".into(), json!(json_type));

        let description = match (param.requirement, default_site_id) {
            (Requirement::SiteId, Some(id)) => {
                format!(
                    "{} Defaults to the configured site {id}.",
                    param.description
                )
            }
            _ => param.description.to_string(),
        };
        prop.insert("description".into(), json!(description));

        // The select argument's choices come from the binding cases.
        let select_choices: Option<Vec<&str>> = match &spec.binding {
            Binding::Select { arg, cases } if *arg == param.name => {
                Some(cases.iter().map(|c| c.value).collect())
            }
            _ => None,
        };
        if let Some(choices) = select_choices {
            prop.insert("enum".into(), json!(choices));
        } else if !param.choices.is_empty() {
            prop.insert("enum".into(), json!(param.choices));
        }

        if let Some(default) = param.default {
            let default_json = match param.kind {
                ParamKind::Integer => default
                    .parse::<i64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| json!(default)),
                _ => json!(default),
            };
            prop.insert("default".into(), default_json);
        }

        if param.kind == ParamKind::Object {
            prop.insert("additionalProperties".into(), json!(true));
        }

        properties.insert(param.name.to_string(), Value::Object(prop));

        let is_required = match param.requirement {
            Requirement::Required => true,
            Requirement::SiteId => default_site_id.is_none(),
            Requirement::Optional => false,
        };
        if is_required {
            required.push(json!(param.name));
        }
    }

    let mut schema = Map::new();
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".into(), Value::Array(required));
    }

    Tool {
        name: spec.name.into(),
        description: Some(spec.description.into()),
        input_schema: Arc::new(schema),
        annotations: None,
        icons: None,
        meta: None,
        output_schema: None,
        title: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(json: Value) -> Map<String, Value> {
        json.as_object().unwrap().clone()
    }

    #[test]
    fn catalog_names_are_unique_and_short() {
        let specs = catalog();
        let mut names: Vec<_> = specs.iter().map(|s| s.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), specs.len(), "duplicate tool names");
        for name in names {
            assert!(
                name.len() < 64,
                "tool name '{name}' too long for MCP clients"
            );
            assert!(name.starts_with("matomo_"), "tool '{name}' missing prefix");
        }
    }

    #[test]
    fn fixed_tool_resolves_with_defaults() {
        let registry = Registry::new(None);
        let inv = registry
            .resolve("matomo_visits_summary", &args(json!({"site_id": 3})))
            .unwrap();
        assert_eq!(inv.method, "VisitsSummary.get");
        assert!(inv.params.contains(&("idSite".into(), "3".into())));
        assert!(inv.params.contains(&("period".into(), "day".into())));
        assert!(inv.params.contains(&("date".into(), "yesterday".into())));
        // Optional segment without default must not be sent.
        assert!(!inv.params.iter().any(|(k, _)| k == "segment"));
    }

    #[test]
    fn select_tool_dispatches_by_report() {
        let registry = Registry::new(Some(1));
        let inv = registry
            .resolve("matomo_pages", &args(json!({"report": "exit_pages"})))
            .unwrap();
        assert_eq!(inv.method, "Actions.getExitPageUrls");
        assert!(inv.params.contains(&("flat".into(), "1".into())));
        assert!(inv.params.contains(&("filter_limit".into(), "20".into())));
    }

    #[test]
    fn select_tool_defaults_to_first_case() {
        let registry = Registry::new(Some(1));
        let inv = registry
            .resolve("matomo_referrers", &args(json!({})))
            .unwrap();
        assert_eq!(inv.method, "Referrers.get");
    }

    #[test]
    fn select_tool_rejects_unknown_case_with_options() {
        let registry = Registry::new(Some(1));
        let err = registry
            .resolve("matomo_geo", &args(json!({"level": "galaxy"})))
            .unwrap_err();
        assert!(err.contains("galaxy"));
        assert!(err.contains("country"));
    }

    #[test]
    fn ecommerce_overview_pins_id_goal() {
        let registry = Registry::new(Some(1));
        let inv = registry
            .resolve("matomo_ecommerce", &args(json!({"report": "overview"})))
            .unwrap();
        assert_eq!(inv.method, "Goals.get");
        assert!(inv
            .params
            .contains(&("idGoal".into(), "ecommerceOrder".into())));
    }

    #[test]
    fn default_site_id_is_injected() {
        let registry = Registry::new(Some(42));
        let inv = registry
            .resolve("matomo_visits_summary", &args(json!({})))
            .unwrap();
        assert!(inv.params.contains(&("idSite".into(), "42".into())));
    }

    #[test]
    fn missing_site_id_without_default_is_helpful() {
        let registry = Registry::new(None);
        let err = registry
            .resolve("matomo_visits_summary", &args(json!({})))
            .unwrap_err();
        assert!(err.contains("matomo_list_sites"));
    }

    #[test]
    fn explicit_site_id_wins_over_default() {
        let registry = Registry::new(Some(1));
        let inv = registry
            .resolve("matomo_visits_summary", &args(json!({"site_id": 7})))
            .unwrap();
        assert!(inv.params.contains(&("idSite".into(), "7".into())));
        assert!(!inv.params.contains(&("idSite".into(), "1".into())));
    }

    #[test]
    fn invalid_period_is_rejected() {
        let registry = Registry::new(Some(1));
        let err = registry
            .resolve(
                "matomo_visits_summary",
                &args(json!({"period": "fortnight"})),
            )
            .unwrap_err();
        assert!(err.contains("fortnight"));
        assert!(err.contains("range"));
    }

    #[test]
    fn boolean_and_numeric_string_coercion() {
        let registry = Registry::new(Some(1));
        let inv = registry
            .resolve(
                "matomo_pages",
                &args(json!({"limit": "50", "site_id": "9"})),
            )
            .unwrap();
        assert!(inv.params.contains(&("filter_limit".into(), "50".into())));
        assert!(inv.params.contains(&("idSite".into(), "9".into())));
    }

    #[test]
    fn raw_tool_passes_params_and_strips_reserved() {
        let registry = Registry::new(None);
        let inv = registry
            .resolve(
                "matomo_api",
                &args(json!({
                    "method": "VisitFrequency.get",
                    "params": {
                        "idSite": 1,
                        "period": "week",
                        "token_auth": "evil",
                        "FORMAT": "XML",
                        "module": "CoreAdminHome",
                        "expanded": true
                    }
                })),
            )
            .unwrap();
        assert_eq!(inv.method, "VisitFrequency.get");
        assert!(inv.params.contains(&("idSite".into(), "1".into())));
        assert!(inv.params.contains(&("expanded".into(), "1".into())));
        assert!(!inv
            .params
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("token_auth")));
        assert!(!inv
            .params
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("format")));
        assert!(!inv
            .params
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("module")));
    }

    #[test]
    fn raw_tool_validates_method_shape() {
        let registry = Registry::new(None);
        for bad in [
            "nodot",
            "Too.Many.Dots",
            "bad chars.get",
            "",
            ".get",
            "Module.",
        ] {
            let err = registry
                .resolve("matomo_api", &args(json!({"method": bad})))
                .unwrap_err();
            assert!(
                err.contains("method"),
                "expected method error for '{bad}', got: {err}"
            );
        }
    }

    #[test]
    fn schema_marks_site_id_required_only_without_default() {
        let without_default = Registry::new(None);
        let with_default = Registry::new(Some(1));

        let find = |reg: &Registry| {
            reg.mcp_tools()
                .into_iter()
                .find(|t| t.name == "matomo_visits_summary")
                .unwrap()
        };

        let schema = find(&without_default);
        let required = schema.input_schema.get("required").unwrap();
        assert!(required.as_array().unwrap().contains(&json!("site_id")));

        let schema = find(&with_default);
        assert!(schema.input_schema.get("required").is_none());
    }

    #[test]
    fn schema_select_arg_gets_enum_from_cases() {
        let registry = Registry::new(Some(1));
        let tool = registry
            .mcp_tools()
            .into_iter()
            .find(|t| t.name == "matomo_devices")
            .unwrap();
        let dimension = tool
            .input_schema
            .get("properties")
            .and_then(|p| p.get("dimension"))
            .unwrap();
        let choices = dimension.get("enum").unwrap().as_array().unwrap();
        assert!(choices.contains(&json!("browser")));
        assert!(choices.contains(&json!("resolution")));
    }
}
