#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::FluxTubeApp;
use fluxtube_diagnostics::init_logging;
use iced::Size;

fn load_dotenv_from_path(path: &std::path::Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            if std::env::var_os(key).is_none() {
                std::env::set_var(key, val);
            }
        }
    }
}

fn load_dotenv_file() {
    load_dotenv_from_path(std::path::Path::new(".env"));
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            load_dotenv_from_path(&exe_dir.join(".env"));
        }
    }
}

fn main() -> iced::Result {
    load_dotenv_file();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke-test") {
        let (app, _task) = FluxTubeApp::bootstrap(true).expect("FluxTube smoke bootstrap failed");
        println!(
            "FluxTube smoke check OK | locale={} | theme={} | profile={:?} | mock_mode={} | videos={}",
            app.loc.current_locale(),
            app.ui.settings.theme,
            app.ui.settings.performance.profile,
            app.ui.is_mock_mode,
            app.ui.home_videos.len()
        );
        return Ok(());
    }

    let _logging = init_logging(None, None);

    iced::application("FluxTube", FluxTubeApp::update, FluxTubeApp::view)
        .subscription(FluxTubeApp::subscription)
        .window_size(Size::new(1280.0, 760.0))
        .run_with(|| {
            FluxTubeApp::bootstrap(false).expect("Failed to initialize FluxTube desktop runtime")
        })
}
