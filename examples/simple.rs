use std::{thread, time};
use win_events::{error::ErrorKind, event::WinLogEvent, reader};

fn main() {
    let mut config = reader::Config::default();

    config.read_oldest = true;
    config.output = reader::Output::Json;

    let r = reader::Reader::init(config).unwrap();

    loop {
        match r.next() {
            Ok(event) => match event {
                WinLogEvent::Xml(xml) => println!("{:?}", xml),

                WinLogEvent::Raw(e) => println!("{:?}", e),

                WinLogEvent::Parsed(e) => println!("{:?}", e),

                WinLogEvent::Json(json) => println!("{}", json),
            },

            Err(err) => match err.kind {
                ErrorKind::NoMoreLogs => {
                    thread::sleep(time::Duration::from_secs(2));
                    continue;
                }
                ErrorKind::Event => {
                    println!("unable to get event {}", err);
                    continue;
                }
                ErrorKind::Subscription => {
                    println!("error occurred in event subscription {}", err);
                    break;
                }
            },
        }
    }

    if let Ok(xml) = r.get_bookmark() {
        println!("xml bookmark: {}", xml)
    };
}
