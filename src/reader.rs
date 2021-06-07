use crate::error::{Error, ErrorKind, Result};
use crate::event::{Event, RawEvent, WinLogEvent};

use bindings::{
    Windows::Win32::System::EventLog::{
        EvtClose, EvtCreateBookmark, EvtFormatMessage, EvtNext, EvtOpenPublisherMetadata,
        EvtRender, EvtSubscribe, EvtUpdateBookmark,
    },
    Windows::Win32::System::SystemServices::{HANDLE, PWSTR},
    Windows::Win32::System::Threading::CreateEventW,
    Windows::Win32::System::WindowsProgramming::CloseHandle,
};

use core::ffi::c_void;
use quick_xml::{events::Event as QuickXmlEvent, Reader as QuickXmlReader};
use std::convert::TryInto;
use std::io::Error as IoError;

const DEFAULT_QUERY: &str = r#"
<QueryList>
    <Query Id='0'>
        <Select Path='Application'>*</Select>
        <Select Path='Security'>*</Select>
        <Select Path='System'>*</Select>
    </Query>
</QueryList>
"#;

const EVT_SUBSCRIBE_TO_FUTURE_EVENTS: u32 = 1;
const EVT_SUBSCRIBE_START_AT_OLDEST_RECORD: u32 = 2;
const EVT_SUBSCRIBE_START_AFTER_BOOKMARK: u32 = 3;

const EVT_FORMAT_MESSAGE_XML: u32 = 9;

const EVT_RENDER_FLAG_EVENT_XML: u32 = 1;
const EVT_RENDER_FLAG_BOOKMARK: u32 = 2;

#[derive(Debug)]
pub struct Config {
    pub read_oldest: bool,
    pub query: String,
    pub bookmark: Option<String>,
    pub output: Output,
}

#[derive(Debug)]
pub enum Output {
    Xml,
    Raw,
    Parsed,
    Json,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            read_oldest: false,
            query: DEFAULT_QUERY.to_string(),
            bookmark: None,
            output: Output::Parsed,
        }
    }
}

pub struct Reader {
    subscription_handle: isize,
    bookmark_handle: isize,
    signal: Option<HANDLE>,
    output: Output,
}

impl Reader {
    pub fn init(config: Config) -> Result<Reader> {
        let mut flag = EVT_SUBSCRIBE_TO_FUTURE_EVENTS;

        if config.read_oldest {
            flag = EVT_SUBSCRIBE_START_AT_OLDEST_RECORD;
        }

        let mut bookmark_handle: isize = 0;

        if let Some(xml) = config.bookmark {
            bookmark_handle = unsafe { EvtCreateBookmark(xml) };
        };

        if bookmark_handle != 0 {
            flag = EVT_SUBSCRIBE_START_AFTER_BOOKMARK;
        } else {
            // TODO: should we error out here?
            println!("unable to create bookmark handler from stored bookmark");
        }

        let signal = unsafe { CreateEventW(std::ptr::null_mut(), false, true, PWSTR::default()) };
        if signal.is_null() || signal.is_invalid() {
            return Err(Error::subscription(
                "unable to create signal",
                IoError::last_os_error(),
            ));
        }
        let subscription_handle = unsafe {
            EvtSubscribe(
                0, //session
                signal,
                PWSTR::default(),
                config.query,
                bookmark_handle,
                std::ptr::null_mut(), //context
                None,                 //callback
                flag,
            )
        };

        if subscription_handle == 0 {
            return Err(Error::subscription(
                "unable to create events subscription",
                IoError::last_os_error(),
            ));
        }

        // if bookmark handle is null then create default bookmark to store new events
        if bookmark_handle == 0 {
            // try to create default bookmark
            bookmark_handle = unsafe { EvtCreateBookmark(PWSTR::default()) };

            if bookmark_handle == 0 {
                // TODO: Close signal & subscription handler??
                return Err(Error::subscription(
                    "unable to create default bookmark handle",
                    IoError::last_os_error(),
                ));
            }
        }

        Ok(Self {
            subscription_handle,
            bookmark_handle,
            signal: Some(signal),
            output: config.output,
        })
    }

    pub fn get_bookmark(&self) -> Result<String> {
        Ok(render_event(
            &self.bookmark_handle,
            EVT_RENDER_FLAG_BOOKMARK,
        )?)
    }

    pub fn next(&self) -> Result<WinLogEvent> {
        let event = next_event(&self.subscription_handle)?;

        if event.is_none() {
            return Err(Error {
                kind: ErrorKind::NoMoreLogs,
                message: "".to_owned(),
            });
        }

        let xml = process_event(&event.unwrap(), &self.bookmark_handle)?;

        unsafe {
            EvtClose(event.unwrap());
        }

        match self.output {
            Output::Xml => Ok(WinLogEvent::Xml(xml)),

            Output::Raw => Ok(WinLogEvent::Raw(xml.try_into()?)),

            Output::Parsed => Ok(WinLogEvent::Parsed(
                TryInto::<RawEvent>::try_into(xml)?.into(),
            )),

            Output::Json => {
                let event: Event = TryInto::<RawEvent>::try_into(xml)?.into();
                match serde_json::to_string(&event) {
                    Ok(json) => Ok(WinLogEvent::Json(json)),
                    Err(err) => Err(Error {
                        kind: ErrorKind::Event,
                        message: err.to_string(),
                    }),
                }
            }
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if let Some(s) = self.signal {
            if !s.is_null() && !s.is_invalid() {
                unsafe {
                    CloseHandle(self.signal);
                }
            }
        }

        if self.subscription_handle != 0 {
            unsafe {
                EvtClose(self.subscription_handle);
            }
        }
        if self.bookmark_handle != 0 {
            unsafe { EvtClose(self.bookmark_handle) };
        }
    }
}

fn process_event(event: &isize, bookmark_handle: &isize) -> Result<String> {
    // update bookmark
    if !unsafe { EvtUpdateBookmark(*bookmark_handle, *event).as_bool() } {
        let err = IoError::last_os_error();
        return Err(Error::event("unable to update bookmark", err));
    }

    let provider = match render_event(event, EVT_RENDER_FLAG_EVENT_XML) {
        Ok(xml) => parse_provider_name(&xml),
        Err(_err) => None,
    };

    let xml = format_message(event, EVT_FORMAT_MESSAGE_XML, provider)?;
    Ok(xml)
}

fn next_event(subscription_handle: &isize) -> Result<Option<isize>> {
    let mut event_count: u32 = 0;
    let mut events: [isize; 1] = [0; 1];

    if unsafe {
        EvtNext(
            *subscription_handle,
            1,
            events.as_mut_ptr(),
            500, // 0.5sec
            0,
            &mut event_count,
        )
        .as_bool()
    } {
        return Ok(Some(events[0]));
    }

    match IoError::last_os_error().raw_os_error() {
        // (1460) ERROR_TIMEOUT | (259) ERROR_NO_MORE_ITEMS | (4317) ERROR_INVALID_OPERATION
        None | Some(1460) | Some(259) | Some(4317) => return Ok(None),

        Some(e) => {
            return Err(Error::subscription(
                "error getting next windows logs event",
                IoError::from_raw_os_error(e),
            ));
        }
    }
}

fn render_event(event_handler: &isize, flag: u32) -> Result<String> {
    // let status;
    let mut buffer_size: u32 = 0;
    let mut buffer_used: u32 = 0;

    // find buffer size
    unsafe {
        EvtRender(
            0,
            *event_handler,
            flag,
            buffer_size,
            std::ptr::null_mut(),
            &mut buffer_used,
            std::ptr::null_mut(),
        )
    };

    if let Some(err) = IoError::last_os_error().raw_os_error() {
        // 122 (0x7A) ERROR_INSUFFICIENT_BUFFER
        if err != 122 {
            return Err(Error::event(
                "unable to render event",
                IoError::from_raw_os_error(err),
            ));
        }
    }

    // set vec length to req buffer size
    buffer_size = buffer_used;

    let mut buf: Vec<u16> = vec![0; buffer_size as usize];

    if !unsafe {
        EvtRender(
            0,
            *event_handler,
            flag,
            buffer_size,
            buf.as_mut_ptr() as *mut c_void,
            &mut buffer_used,
            std::ptr::null_mut(),
        )
        .as_bool()
    } {
        return Err(Error::event(
            "unable to render event",
            IoError::last_os_error(),
        ));
    }
    Ok(String::from_utf16_lossy(&buf[..])
        .trim_matches(char::from(0))
        .to_string())
}

fn format_message(event_hander: &isize, flag: u32, publisher: Option<String>) -> Result<String> {
    let mut publisher_metadata = 0;

    if publisher.is_some() {
        publisher_metadata =
            unsafe { EvtOpenPublisherMetadata(0, publisher.unwrap(), PWSTR::default(), 0, 0) }
    }

    // let status;
    let mut buffer_size: u32 = 0;
    let mut buffer_used: u32 = 0;

    // find buffer size
    unsafe {
        EvtFormatMessage(
            publisher_metadata,
            *event_hander,
            0,
            0,
            std::ptr::null_mut(),
            flag,
            buffer_size,
            PWSTR::default(),
            &mut buffer_used,
        )
    };

    if let Some(err) = IoError::last_os_error().raw_os_error() {
        if err != 122 {
            return Err(Error::event(
                "unable to render event",
                IoError::from_raw_os_error(err),
            ));
        }
    }

    // set vec length to req buffer size
    buffer_size = buffer_used;

    let mut buf: Vec<u16> = vec![0; buffer_size as usize];

    if !unsafe {
        EvtFormatMessage(
            publisher_metadata,
            *event_hander,
            0,
            0,
            std::ptr::null_mut(),
            flag,
            buffer_size,
            PWSTR(buf.as_mut_ptr()),
            &mut buffer_used,
        )
        .as_bool()
    } {
        return Err(Error::event(
            "unable to render event",
            IoError::last_os_error(),
        ));
    }
    Ok(String::from_utf16_lossy(&buf[..])
        .trim_matches(char::from(0))
        .to_string())
}

fn parse_provider_name(xml: &str) -> Option<String> {
    let mut reader = QuickXmlReader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(QuickXmlEvent::Empty(ref e)) => match e.name() {
                b"Provider" => {
                    for a in e.attributes() {
                        if let Ok(att) = a {
                            if b"Name" == att.key {
                                if let Ok(v) = att.unescape_and_decode_value(&reader) {
                                    return Some(v);
                                }
                            }
                        }
                    }
                    break;
                }
                _ => (),
            },
            Ok(QuickXmlEvent::Eof) => break,
            _ => (),
        }
    }
    None
}
