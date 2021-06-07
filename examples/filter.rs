use std::{thread, time};
use win_events::{error::ErrorKind, event::WinLogEvent, filter, filter::Level, reader};

fn main() {
    let f1 = filter::Config {
        channel: "Application".into(),

        // to exclude events use '-' first in event number or range
        event_id: Some("1,16384,-3433,100-200,-300-400".to_string()),
        level: Some(vec![Level::Information, Level::Warning]),
        ignore_older: Some(43200),
        provider: None,
    };

    let f2 = filter::Config {
        channel: "Security".into(),
        level: Some(vec![Level::Error, Level::Information]),
        ..Default::default()
    };

    let query = filter::build_query(vec![f1, f2]);

    let config = reader::Config {
        read_oldest: true,
        query,
        bookmark: Some(
            r#"<BookmarkList>
            <Bookmark Channel='Application' RecordId='6672'/>
            <Bookmark Channel='Security' RecordId='30044' IsCurrent='true'/>
          </BookmarkList>"#
                .to_owned(),
        ),
        output: reader::Output::Parsed,
    };

    let r = reader::Reader::init(config).unwrap();

    loop {
        match r.next() {
            Ok(event) => {
                if let WinLogEvent::Parsed(e) = event {
                    println!("{} - {}", e.event_id, e.record_id)
                }
            }

            Err(err) => match err.kind {
                ErrorKind::NoMoreLogs => {
                    // wait and check again for new events
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
