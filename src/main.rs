mod cache;
mod cbz;
mod ui;

use gio::ApplicationFlags;
use gtk4::prelude::*;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use ui::ManhwaWindow;

/// Hand-rolled parse of the dewey-facing options (`--file <path>`, `--page <n>`, `--mode <mode>`, `--storage-profile <prof>`).
/// Continuum itself only accepts a positional file, so these are consumed here
/// and stripped before the GTK/GApplication option parser sees argv.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    file: Option<PathBuf>,
    page: Option<i64>,
    mode: Option<String>,
    storage_profile: Option<String>,
}

fn parse_cli_args(raw: &[String]) -> CliArgs {
    let mut cli = CliArgs {
        file: None,
        page: None,
        mode: None,
        storage_profile: None,
    };
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--file" => {
                if let Some(v) = raw.get(i + 1) {
                    cli.file = Some(PathBuf::from(v));
                    i += 1;
                }
            }
            "--page" => {
                if let Some(v) = raw.get(i + 1) {
                    cli.page = v.parse().ok();
                    i += 1;
                }
            }
            "--mode" => {
                if let Some(v) = raw.get(i + 1) {
                    cli.mode = Some(v.to_string());
                    i += 1;
                }
            }
            "--storage-profile" => {
                if let Some(v) = raw.get(i + 1) {
                    cli.storage_profile = Some(v.to_string());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    cli
}

/// argv for `run_with_args`: argv[0] plus positionals, with `--file/--page/--mode/--storage-profile`
/// (and their values) removed so GLib's option parser accepts them.
fn filter_args(raw: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--file" | "--page" | "--mode" | "--storage-profile" => i += 2, // skip option + value
            other => {
                out.push(other.to_string());
                i += 1;
            }
        }
    }
    out
}

fn main() -> glib::ExitCode {
    libadwaita::init().expect("Failed to initialize Libadwaita");

    let raw: Vec<String> = env::args().collect();
    let cli = parse_cli_args(&raw);
    let run_args = filter_args(&raw);

    // NON_UNIQUE: each invocation runs as its own process instead of handing
    // off to a running instance over D-Bus, so callers (dewey) get a fresh
    // window and a reliable close payload from the process they spawned.
    let app = libadwaita::Application::builder()
        .application_id("dev.continuum.ManhwaReader")
        .flags(ApplicationFlags::HANDLES_OPEN | ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_startup(|_| {
        load_custom_styles();
    });

    let cli_file = Rc::new(cli.file);
    let cli_page = cli.page;
    let cli_mode = Rc::new(cli.mode);
    let cli_storage_profile = Rc::new(cli.storage_profile);

    let cli_file_activate = cli_file.clone();
    let cli_mode_activate = cli_mode.clone();
    let cli_profile_activate = cli_storage_profile.clone();
    app.connect_activate(move |app| {
        open_window(
            app,
            cli_file_activate.as_ref().clone(),
            cli_page,
            cli_mode_activate.as_ref().as_deref(),
            cli_profile_activate.as_ref().as_deref(),
        );
    });

    let cli_file_open = cli_file;
    let cli_mode_open = cli_mode;
    let cli_profile_open = cli_storage_profile;
    app.connect_open(move |app, files, _| {
        let positional = files.first().and_then(|f| f.path());
        let path = cli_file_open.as_ref().clone().or(positional);
        open_window(
            app,
            path,
            cli_page,
            cli_mode_open.as_ref().as_deref(),
            cli_profile_open.as_ref().as_deref(),
        );
    });

    app.run_with_args(&run_args)
}

fn open_window(
    app: &libadwaita::Application,
    path: Option<PathBuf>,
    page: Option<i64>,
    mode: Option<&str>,
    storage_profile: Option<&str>,
) {
    if let Some(active_win) = app.active_window() {
        active_win.present();
        return;
    }

    let manhwa_window = ManhwaWindow::new(app);
    if let Some(m) = mode {
        let reading_mode = crate::ui::page_widget::ReadingMode::from_str_loose(m);
        manhwa_window.reader.set_reading_mode(reading_mode);
        manhwa_window.update_window_size_for_mode(reading_mode);
    }
    if let Some(prof) = storage_profile {
        manhwa_window.reader.set_storage_profile(prof);
    } else if let Some(ref p) = path {
        let p_str = p.to_string_lossy().to_lowercase();
        if p_str.starts_with("/media/")
            || p_str.starts_with("/run/media/")
            || p_str.starts_with("/mnt/")
            || p_str.contains("/usb")
        {
            manhwa_window.reader.set_storage_profile("usb");
        }
    }
    if let Some(p) = path.filter(|p| p.exists()) {
        let _ = manhwa_window.reader.load_initial_file(p);
        let mode = *manhwa_window.reader.reading_mode.borrow();
        manhwa_window.update_window_size_for_mode(mode);
        if let Some(page) = page.filter(|p| *p > 0) {
            manhwa_window.reader.jump_to_page(page as usize);
        }
    }
    manhwa_window.window.present();
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_and_page_flags() {
        let raw = vec![
            "continuum".to_string(),
            "--file".to_string(),
            "/a/x.cbz".to_string(),
            "--page".to_string(),
            "42".to_string(),
            "--mode".to_string(),
            "manga".to_string(),
            "--storage-profile".to_string(),
            "usb".to_string(),
        ];
        let cli = parse_cli_args(&raw);
        assert_eq!(cli.file, Some(PathBuf::from("/a/x.cbz")));
        assert_eq!(cli.page, Some(42));
        assert_eq!(cli.mode, Some("manga".to_string()));
        assert_eq!(cli.storage_profile, Some("usb".to_string()));
        // GLib must never see our private flags.
        assert_eq!(filter_args(&raw), vec!["continuum".to_string()]);
    }

    #[test]
    fn keeps_positional_and_ignores_page_without_value() {
        let raw = vec![
            "continuum".to_string(),
            "/a/y.cbz".to_string(),
            "--page".to_string(),
            "7".to_string(),
        ];
        let cli = parse_cli_args(&raw);
        assert_eq!(cli.file, None);
        assert_eq!(cli.page, Some(7));
        assert_eq!(cli.mode, None);
        assert_eq!(
            filter_args(&raw),
            vec!["continuum".to_string(), "/a/y.cbz".to_string()]
        );
        // --page with no value at end must not panic.
        let raw2 = vec!["continuum".to_string(), "--page".to_string()];
        assert_eq!(filter_args(&raw2), vec!["continuum".to_string()]);
        assert_eq!(parse_cli_args(&raw2).page, None);
    }

    #[test]
    fn test_load_custom_styles() {
        if gtk4::init().is_err() {
            return;
        }
        load_custom_styles();
    }

    #[test]
    fn test_manga_page_centering() {
        if gtk4::init().is_err() {
            return;
        }
        let file = std::path::PathBuf::from(
            "/home/omary/Documents/Dewey/Manga/Action/Dandadan/[0245]_Chapter_1.cbz",
        );
        if !file.exists() {
            return;
        }
        let app = libadwaita::Application::builder()
            .application_id("dev.continuum.centering_test")
            .build();
        let manhwa_window = ManhwaWindow::new(&app);
        manhwa_window
            .reader
            .set_reading_mode(crate::ui::page_widget::ReadingMode::ContinuousHorizontal);
        manhwa_window
            .update_window_size_for_mode(crate::ui::page_widget::ReadingMode::ContinuousHorizontal);
        manhwa_window.window.present();
        manhwa_window.reader.load_initial_file(file).unwrap();

        let ctx = glib::MainContext::default();
        for _ in 0..50 {
            ctx.iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let reader = &manhwa_window.reader;
        let hadj = reader.scrolled_window.hadjustment();

        let g2k = reader.global_to_key.borrow();
        let widgets = reader.page_widgets.borrow();
        for page_num in [0, 1, 2, 3, 4] {
            if let Some(key) = g2k.get(page_num) {
                if let Some(pw) = widgets.get(key) {
                    *reader.focused_page_idx.borrow_mut() = page_num;
                    reader.center_focused_page();

                    let b_clamp = pw.container.compute_bounds(&reader.clamp).unwrap();
                    let page_center_in_clamp = b_clamp.x() as f64 + b_clamp.width() as f64 / 2.0;
                    let page_center_on_screen = page_center_in_clamp - hadj.value();
                    let viewport_center = hadj.page_size() / 2.0;

                    assert!(
                        (page_center_on_screen - viewport_center).abs() <= 1.0,
                        "Page {} must be centered: center_on_screen={}, viewport_center={}",
                        page_num,
                        page_center_on_screen,
                        viewport_center
                    );
                }
            }
        }
    }
}

fn load_custom_styles() {
    let provider = gtk4::CssProvider::new();
    let css = r#"
        window {
            background-color: #121212;
        }

        .page-container {
            margin: 0;
            padding: 0;
            background-color: #121212;
        }

        picture {
            margin: 0;
            padding: 0;
            border: none;
        }

        .page-placeholder {
            background-color: #1a1a1a;
            border-radius: 8px;
            margin: 0;
            padding: 24px;
            border: 1px dashed #333333;
        }

        .chapter-banner {
            background-color: #161616;
            padding: 16px 0;
        }

        .chapter-line {
            background-color: #3b3b4f;
            min-height: 2px;
            margin: 12px 0;
            opacity: 0.8;
        }

        .accent-card {
            background-color: #242424;
            padding: 12px 24px;
            border-radius: 12px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
        }

        scrolledwindow {
            background-color: #121212;
        }

        /* Manga horizontal reading mode: edge vignette shadow casting over side peek pages */
        scrolledwindow.manga-fade-overlay {
            background-color: #0c0c0e;
            box-shadow: inset 80px 0 60px -30px rgba(0, 0, 0, 0.95),
                        inset -80px 0 60px -30px rgba(0, 0, 0, 0.95);
        }

        .manga-page {
            margin: 0;
            padding: 0;
            border-radius: 4px;
            box-shadow: 0 6px 24px rgba(0, 0, 0, 0.75);
            transition: opacity 220ms ease, box-shadow 220ms ease;
        }

        .manga-page-focused {
            opacity: 1.0;
            box-shadow: 0 16px 52px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.12);
        }

        .manga-page-side {
            opacity: 0.35;
            box-shadow: 0 4px 16px rgba(0, 0, 0, 0.85);
        }
    "#;

    provider.load_from_string(css);

    if let Some(display) = gdk4::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        gtk4::Window::set_default_icon_name("dev.continuum.ManhwaReader");
    }
}
