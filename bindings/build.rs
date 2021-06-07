fn main() {
    windows::build!(
        Windows::Win32::System::EventLog::{
            EvtClose, EvtCreateBookmark, EvtFormatMessage, EvtNext, EvtOpenPublisherMetadata,
            EvtRender, EvtSubscribe, EvtUpdateBookmark,
        },
        Windows::Win32::System::SystemServices::{ HANDLE, PWSTR},
        Windows::Win32::System::Threading::CreateEventW,
        Windows::Win32::System::WindowsProgramming::CloseHandle,
    );
}
