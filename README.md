# win-events

`win-events` is a rust library to pull windows log events. Following output format is supported

  * xml string
  * [Raw](src/event.rs)
  * [Parsed](src/event.rs)
  * Json string


[examples](examples)


## filter package

filter package provides functionality to build filter xml query string. This query is used in reader subscription.

 * name of `channel` to monitor for events. To get list of available channel run `Get-WinEvent -ListLog * | format-list -property LogName`.
 
 * `event_id` is a comma-separated list of event IDs to include or exclude. The accepted values are single event ID to include, a range of event IDs to include (4600-5300) and To exclude, add a minus sign first. For example `1,3,5-99,-76,-300-400`.

 * `level` is list of log levels to include

 * `ignore_older` takes number of second, if specified, events that are older than the specified amount of time will be excluded.

 * `provider` A list of providers (source names) to include.

```rs
let f1 = filter::Config {
        channel: "Application".to_string(),
        event_id: Some("1,16384,-3433,100-200,-300-400".to_string()),
        level: Some(vec![Level::Information, Level::Warning]),
        ignore_older: Some(43200),
        provider: None,
};

let query = filter::build_query(vec![f1]);
```

## reader package

reader package provides functions to pull events based on query provided. If `read_oldest` set to true existing events are returned or only future will be returned.

`next` will pull one event at a time. `NoMoreLogs` will be returned when there are no logs to pull for given query. 

`get_bookmark` will return bookmark xml string, which can be used by caller in next run to specify starting point to pull events.

```rs
use std::{thread, time};
use win_events::{error::ErrorKind, event::WinLogEvent, reader};

fn main() {
    let mut config = reader::Config::default();

    let r = reader::Reader::init(config).unwrap();

    loop {
        match r.next() {
            Ok(event) => println!("{:?}", event),

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
```
