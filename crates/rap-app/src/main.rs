#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// aws-lc-sys (TLS for Google Cast) exports a few entropy functions, so the MSVC
// linker reports creating an import library next to the executable. Harmless.
#![allow(linker_messages)]
mod album;
mod app;
mod assets;
mod audio_preferences;
mod bluetooth_preferences;
mod catalog;
mod collection;
mod filters;
mod history;
mod i18n;
mod indexing;
mod input;
mod keys;
mod notices;
mod now_playing;
mod output_watch;
mod placement;
mod playback;
mod player;
mod playlist_view;
mod playlists;
mod preferences;
mod queue_view;
mod resume;
mod scale;
mod scrollbar;
mod service;
mod slider;
mod system;
mod theme;
mod title_bar;
mod track_list;
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
    // Covers of files played without indexing only live for one session.
    let _ = std::fs::remove_dir_all(paths.covers.join("loose"));
    tracing::info!("RAP starting");
    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx| {
            assets::Assets::load_fonts(cx).expect("Cannot load the bundled Inter fonts");
            input::bind_keys(cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let window_bounds = placement::restore(settings.window, cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(window_bounds),
                    window_min_size: Some(placement::MIN_SIZE),
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        title: Some("RAP".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let app = cx.new(|cx| app::Rap::new(settings, paths, window, cx));
                    let owner = app.downgrade();
                    window.on_window_should_close(cx, move |window, cx| {
                        let _ = owner.update(cx, |app, cx| app.request_close(window, cx));
                        false
                    });
                    app.read(cx).focus.focus(window);
                    app
                },
            )
            .expect("Cannot open RAP window");
            cx.activate(true);
        });
    Ok(())
}
