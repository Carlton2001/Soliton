#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod album;
mod app;
mod assets;
mod catalog;
mod filters;
mod i18n;
mod indexing;
mod input;
mod playback;
mod scrollbar;
mod service;
mod theme;
mod views;
mod widgets;

use gpui::*;
use rap_core::config::Settings;
use rap_platform::AppPaths;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let paths = match std::env::var_os("RAP_DATA_DIR") {
        Some(root) => AppPaths::at(root.into()),
        None => AppPaths::discover()?,
    };
    paths.create()?;
    let settings = Settings::load(&paths.settings)?;
    // Migrations and initial configuration run before the UI event loop exists.
    drop(rap_library::Library::open(&paths.root)?);
    tracing::info!("RAP starting");
    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx| {
            input::bind_keys(cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let bounds = Bounds::centered(None, size(px(1180.), px(780.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(880.), px(600.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("RAP".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let app = cx.new(|cx| app::Rap::new(settings, paths, cx));
                    app.read(cx).focus.focus(window);
                    app
                },
            )
            .expect("Cannot open RAP window");
            cx.activate(true);
        });
    Ok(())
}
