use chrono::{DateTime, Utc};
use core::convert::Into;
use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeValue;
use std::collections::HashMap;
use std::convert::From;
use std::convert::TryFrom;

use crate::error::{Error, ErrorKind};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum WinLogEvent {
    Xml(String),
    Raw(RawEvent),
    Parsed(Event),
    Json(String),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RawEvent {
    pub system: System,
    pub event_data: Option<EventData>,
    pub user_data: Option<HashMap<String, UserData>>,
    pub rendering_info: Option<RenderingInfo>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct System {
    pub provider: Provider,
    #[serde(rename = "EventID")]
    pub event_id: EventID,
    pub version: u8,
    pub level: u8,
    pub task: u16,
    pub opcode: u8,
    pub keywords: String,
    pub time_created: TimeCreated,
    #[serde(rename = "EventRecordID")]
    pub event_record_id: Option<u64>,
    pub correlation: Correlation,
    pub execution: Execution,
    pub channel: String,
    pub computer: String,
    pub security: Security,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TimeCreated {
    pub system_time: Option<String>,
    pub raw_time: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Security {
    #[serde(rename = "UserID")]
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Provider {
    pub name: Option<String>,
    #[serde(rename = "GUID")]
    pub guid: Option<String>,
    pub event_source_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct EventID {
    pub qualifiers: Option<u16>,
    #[serde(rename = "$value")]
    pub id: u32,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Correlation {
    #[serde(rename = "ActivityID")]
    pub activity_id: Option<String>,
    #[serde(rename = "RelatedActivityID")]
    pub related_activity_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Execution {
    #[serde(rename = "ProcessID")]
    pub process_id: u32,
    #[serde(rename = "ThreadID")]
    pub thread_id: u32,

    #[serde(rename = "ProcessorID")]
    pub processor_id: Option<u8>,
    #[serde(rename = "SessionID")]
    pub session_id: Option<u32>,
    pub kernel_time: Option<u32>,
    pub user_time: Option<u32>,
    pub processor_time: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct EventData {
    pub data: Option<Vec<Data>>,
    pub binary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Data {
    pub name: Option<String>,
    #[serde(rename = "Type")]
    pub data_type: Option<String>,
    #[serde(rename = "$value")]
    pub value: Option<String>,
}

pub type UserData = HashMap<String, String>;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RenderingInfo {
    pub message: String,
    pub level: String,
    pub opcode: String,
    pub task: String,
    pub channel: String,
    pub provider: String,
    pub keywords: Keywords,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Keywords {
    pub keyword: Option<Vec<String>>,
}

impl TryFrom<String> for RawEvent {
    type Error = Error;

    fn try_from(xml: String) -> Result<Self, Error> {
        match quick_xml::de::from_str::<RawEvent>(&xml) {
            Ok(e) => Ok(e),
            Err(err) => Err(Error {
                kind: ErrorKind::Event,
                message: err.to_string(),
            }),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub record_id: u64,
    pub provider_name: String,
    pub provider_guid: String,
    pub source_name: String,
    pub system_time: Option<DateTime<Utc>>,

    pub event_id: u32,
    pub computer_name: String,
    pub activity_id: String,
    pub channel: String,
    pub level: String,
    pub opcode: String,

    pub task: String,
    pub message: String,

    pub process_id: u32,
    pub thread_id: u32,

    pub user: HashMap<String, SerdeValue>,

    pub keywords: Vec<String>,

    pub event_data: HashMap<String, String>,
    pub user_data: HashMap<String, String>,
}

impl From<RawEvent> for Event {
    fn from(raw_event: RawEvent) -> Self {
        let mut event: Event = Event::default();

        event.event_id = raw_event.system.event_id.id;
        event.computer_name = raw_event.system.computer;
        event.channel = raw_event.system.channel;
        event.process_id = raw_event.system.execution.process_id;
        event.thread_id = raw_event.system.execution.thread_id;

        if let Some(t) = raw_event.system.time_created.system_time {
            if let Ok(t) = DateTime::parse_from_rfc3339(&t) {
                event.system_time = Some(t.with_timezone(&Utc));
            }
        }

        if let Some(id) = raw_event.system.event_record_id {
            event.record_id = id;
        }

        if let Some(name) = raw_event.system.provider.name {
            event.provider_name = name;
        }
        if let Some(guid) = raw_event.system.provider.guid {
            event.provider_guid = guid;
        }
        if let Some(name) = raw_event.system.provider.event_source_name {
            event.source_name = name;
        }

        if let Some(id) = raw_event.system.correlation.activity_id {
            event.activity_id = id;
        }

        if let Some(rend_info) = raw_event.rendering_info {
            event.level = rend_info.level;
            event.opcode = rend_info.opcode;
            event.task = rend_info.task;
            event.message = rend_info.message;

            if let Some(keys) = rend_info.keywords.keyword {
                event.keywords = keys;
            }
        }

        if let Some(id) = raw_event.system.security.user_id {
            event
                .user
                .insert("identifier".into(), SerdeValue::String(id));
        }

        if let Some(raw_event_data) = raw_event.event_data {
            // let mut event_data = HashMap::new();

            if let Some(data) = raw_event_data.data {
                for (i, d) in data.iter().enumerate() {
                    if d.value.is_some() {
                        match &d.name {
                            Some(n) => event
                                .event_data
                                .insert(n.to_string(), d.value.to_owned().unwrap()),
                            None => event
                                .event_data
                                .insert(format!("param{}", i), d.value.to_owned().unwrap()),
                        };
                    }
                }
            }

            if let Some(bin) = raw_event_data.binary {
                event.event_data.insert("binary".into(), bin);
            }
        }

        if let Some(raw_user_data) = raw_event.user_data {
            for (_, values) in raw_user_data {
                event.user_data.extend(values);
            }
        }

        event
    }
}
