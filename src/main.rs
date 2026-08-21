mod cache;
mod cbz;
mod ui;

use gio::ApplicationFlags;
use gtk4::prelude::*;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use ui::ManhwaWindow;

/// Hand-rolled parse of the dewey-facing options (`--file <path>`, `--page <n>`).
/// Continuum itself only accepts a positional file, so these are consumed here
/// and stripped before the GTK/GApplication option parser sees argv.
struct CliArgs {
    file: Option<PathBuf>,
    page: Option<i64>,
}

fn parse_cli_args(raw: &[String]) -> CliArgs {
    let mut cli = CliArgs {
        file: None,
        page: None,
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
            _ => {}
        }
        i += 1;
    }
    cli
}

/// argv for `run_with_args`: argv[0] plus positionals, with `--file/--page`
/// (and their values) removed so GLib's option parser accepts them.
fn filter_args(raw: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--file" | "--page" => i += 2, // skip option + value
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
    let cli_file_activate = cli_file.clone();
    app.connect_activate(move |app| {
        open_window(app, cli_file_activate.as_ref().clone(), cli_page);
    });

    let cli_file_open = cli_file.clone();
    app.connect_open(move |app, files, _| {
        let positional = files.first().and_then(|f| f.path());
        let path = cli_file_open.as_ref().clone().or(positional);
        open_window(app, path, cli_page);
    });

    app.run_with_args(&run_args)
}

fn open_window(app: &libadwaita::Application, path: Option<PathBuf>, page: Option<i64>) {
    if let Some(active_win) = app.active_window() {
        active_win.present();
        return;
    }

    let manhwa_window = ManhwaWindow::new(app);
    if let Some(p) = path.filter(|p| p.exists()) {
        let _ = manhwa_window.reader.load_initial_file(p);
        if let Some(page) = page.filter(|p| *p > 0) {
            manhwa_window.reader.jump_to_page(page as usize);
        }
    }
    manhwa_window.window.present();
}

#[cfg(test)]
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
        ];
        let cli = parse_cli_args(&raw);
        assert_eq!(cli.file, Some(PathBuf::from("/a/x.cbz")));
        assert_eq!(cli.page, Some(42));
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
        assert_eq!(
            filter_args(&raw),
            vec!["continuum".to_string(), "/a/y.cbz".to_string()]
        );
        // --page with no value at end must not panic.
        let raw2 = vec!["continuum".to_string(), "--page".to_string()];
        assert_eq!(filter_args(&raw2), vec!["continuum".to_string()]);
        assert_eq!(parse_cli_args(&raw2).page, None);
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

        .manga-page {
            margin: 0 16px;
            border-left: 2px solid #2e2e42;
            border-right: 2px solid #2e2e42;
            border-radius: 6px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7);
        }

        .manga-fade-overlay {
            mask-image: linear-gradient(to right, transparent 0%, black 14%, black 86%, transparent 100%);
            -webkit-mask-image: linear-gradient(to right, transparent 0%, black 14%, black 86%, transparent 100%);
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
