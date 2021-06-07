use core::convert::Into;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub channel: String,
    pub level: Option<Vec<Level>>,
    pub event_id: Option<String>,
    pub ignore_older: Option<u64>, // secs
    pub provider: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Level {
    LogAlways,
    Critical,
    Error,
    Warning,
    Information,
    Verbose,
}

impl Level {
    pub fn value(&self) -> String {
        match *self {
            Level::LogAlways => "Level=0".into(),
            Level::Critical => "Level=1".into(),
            Level::Error => "Level=2".into(),
            Level::Warning => "Level=3".into(),
            Level::Information => "Level=4".into(),
            Level::Verbose => "Level=5".into(),
        }
    }
}

pub fn build_query(filters: Vec<Config>) -> String {
    let mut queries: Vec<String> = Vec::new();

    filters.iter().for_each(|f| {
        let mut filters: Vec<String> = Vec::new();

        if let Some(providers) = f.provider.as_ref() {
            filters.push(build_provider(providers));
        }

        if let Some(levels) = f.level.as_ref() {
            filters.push(build_level(levels));
        }

        if let Some(ids) = f.event_id.as_ref() {
            filters.push(build_include_event_id(ids));
        }

        if let Some(sec) = f.ignore_older {
            filters.push(build_ignore_old(sec));
        }

        let mut filter_str: String = String::new();

        if filters.len() > 0 {
            filter_str = format!("[System[{}]]", filters.join(" and "))
        }

        queries.push(format!(
            "<Select Path=\"{}\">*{}</Select>",
            f.channel, filter_str
        ));

        if let Some(ids) = f.event_id.as_ref() {
            queries.push(format!(
                "<Suppress Path=\"{}\">*[System[{}]]</Suppress>",
                f.channel,
                build_exclude_event_id(ids)
            ))
        }
    });

    format!(
        "<QueryList><Query Id=\"0\">{}</Query></QueryList>",
        queries.join("")
    )
    .into()
}

// Provider[@Name='.NET Runtime Optimization Service' or @Name='Microsoft-Windows-All-User-Install-Agent']
fn build_provider(providers: &Vec<String>) -> String {
    let s: Vec<String> = providers.iter().map(|p| format!("@Name='{}'", p)).collect();

    format!("Provider[{}]", s.join(" or ")).into()
}

fn build_ignore_old(sec: u64) -> String {
    return format!("TimeCreated[timediff(@SystemTime) &lt;= {}]", sec * 1000);
}

// (Level=1 or Level=3 or Level=4 or Level=0)
fn build_level(levels: &Vec<Level>) -> String {
    let mut s: Vec<String> = levels.iter().map(|l| l.value()).collect();

    // windows event filter includes level 0 when info is selected
    if s.contains(&"Level=4".to_string()) {
        s.push("Level=0".into())
    }

    format!("({})", s.join(" or ")).into()
}

fn build_include_event_id(ids: &str) -> String {
    build_event_id(ids.split(",").filter(|i| !i.starts_with("-")).collect())
}

fn build_exclude_event_id(ids: &str) -> String {
    build_event_id(
        ids.split(",")
            .filter(|i| i.starts_with("-"))
            .map(|i| i.trim_start_matches("-"))
            .collect(),
    )
}

// (EventID=2 or EventID=4 or EventID=5 or  (EventID &gt;= 7 and EventID &lt;= 80))
fn build_event_id(t: Vec<&str>) -> String {
    let mut s: Vec<String> = Vec::new();

    t.iter().for_each(|id| {
        // check if string is a valid id u32
        if let Ok(i) = id.parse::<u32>() {
            s.push(format!("EventID={}", i));
        }

        // Check if string is a range 35-59
        let ids: Vec<&str> = id.split("-").collect();
        // validation
        if ids.len() == 2 && ids[0].parse::<u32>().is_ok() && ids[1].parse::<u32>().is_ok() {
            let start = ids[0].parse::<u32>().unwrap();
            let end = ids[1].parse::<u32>().unwrap();
            if start < end {
                s.push(format!(
                    "(EventID &gt;= {} and EventID &lt;= {})",
                    start, end
                ));
            }
        }
    });
    format!("({})", s.join(" or ")).into()
}
