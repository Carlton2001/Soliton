//! Diagnostic entry point for testing the library without a graphics/audio device.
use soliton_library::{Cancellation, Library};
use soliton_platform::AppPaths;

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let paths = if let Some(root) = arguments.next() {
        AppPaths::at(root.into())
    } else {
        AppPaths::discover()?
    };
    paths.create()?;
    let mut library = Library::open(&paths.root)?;
    if let Some(source) = arguments.next() {
        let id = library.add_source(std::path::Path::new(&source))?;
        let report = library.scan(id, &Cancellation::default(), |_| {})?;
        println!("{report:#?}");
    }
    let snapshot = library.snapshot()?;
    for album in &snapshot.albums {
        println!(
            "{} / {} — {} tracks, {} editions",
            album.artist,
            album.title,
            album.tracks().count(),
            album.editions.len()
        );
    }
    println!(
        "{} sources, {} albums",
        snapshot.sources.len(),
        snapshot.albums.len()
    );
    Ok(())
}
