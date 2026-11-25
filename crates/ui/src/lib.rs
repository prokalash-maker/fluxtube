pub mod messages;
pub mod state;
pub mod views;

pub use messages::{AuthTab, ChannelTab, Route, SettingsTab, UiMessage};
pub use state::{
    extract_chapters, format_relative_age, format_seconds_hms, DeviceAuthState, SidebarItemConfig,
    UiState,
};
pub use views::{render_app, route_label};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_chapters_and_time_formatting() {
        let desc = "Overview\n0:00 Introduction\n2:15 Memory Profiling\n1:02:05 Deep Dive\n";
        let ch = extract_chapters(desc);
        assert_eq!(ch.len(), 3);
        assert_eq!(ch[0], (0, "Introduction".to_string()));
        assert_eq!(ch[1], (135, "Memory Profiling".to_string()));
        assert_eq!(ch[2], (3725, "Deep Dive".to_string()));
        assert_eq!(format_seconds_hms(3725), "1:02:05");
    }
}
