//! Embeds the RAP logo as the Windows executable icon.
//!
//! All six palettes are rasterized from assets/logos at build time.
//! Resource 1 is the default Graphite/Mint icon; resources 2..7 follow PALETTES.

use std::{env, fs, path::PathBuf};

const PALETTES: [&str; 6] = [
    "blue",
    "teal",
    "violet",
    "copper",
    "graphite-mint",
    "ivory-ink",
];
const SIZES: [u32; 7] = [16, 20, 24, 32, 48, 64, 256];

fn main() {
    println!("cargo:rerun-if-env-changed=RAP_UPDATE_REPOSITORY");
    println!("cargo:rerun-if-env-changed=RAP_UPDATE_PUBLIC_KEY");
    for palette in PALETTES {
        println!("cargo:rerun-if-changed=../../assets/logos/{palette}.svg");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let mut resources = String::new();
    for (index, palette) in PALETTES.iter().enumerate() {
        let ico = out.join(format!("rap-{palette}.ico"));
        fs::write(&ico, ico_bytes(palette)).expect("write icon");
        let ico_path = ico.display().to_string().replace('\\', "/");
        if *palette == "graphite-mint" {
            resources.push_str(&format!("1 ICON \"{ico_path}\"\n"));
        }
        resources.push_str(&format!("{} ICON \"{ico_path}\"\n", index + 2));
    }
    let version = env::var("CARGO_PKG_VERSION").expect("package version");
    let numeric = version.split('.').take(3).collect::<Vec<_>>().join(",");
    resources.push_str(&format!(
        r#"
1 VERSIONINFO
FILEVERSION {numeric},0
PRODUCTVERSION {numeric},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "RAP - Rust Audio Player\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "ProductName", "RAP\0"
      VALUE "ProductVersion", "{version}\0"
      VALUE "OriginalFilename", "rap.exe\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    ));
    let rc = out.join("rap.rc");
    fs::write(&rc, resources).expect("write rap.rc");
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_required()
        .expect("compile icon resource");
}

#[cfg(windows)]
fn ico_bytes(palette: &str) -> Vec<u8> {
    let svg = fs::read(format!("../../assets/logos/{palette}.svg")).expect("read logo");
    let tree = resvg::usvg::Tree::from_data(&svg, &Default::default()).expect("parse logo");
    let images: Vec<Vec<u8>> = SIZES.iter().map(|&size| dib(&tree, size)).collect();

    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * SIZES.len() as u32;
    for (&size, image) in SIZES.iter().zip(&images) {
        let dim = if size >= 256 { 0 } else { size as u8 };
        ico.extend_from_slice(&[dim, dim, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for image in images {
        ico.extend_from_slice(&image);
    }
    ico
}

#[cfg(not(windows))]
fn ico_bytes(_: &str) -> Vec<u8> {
    panic!("Windows icons can only be built on a Windows host");
}

/// Renders the logo centered on a transparent square, as a 32-bit icon DIB.
#[cfg(windows)]
fn dib(tree: &resvg::usvg::Tree, size: u32) -> Vec<u8> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("allocate icon");
    // Fit the drawing's own bounds, not the viewBox, to drop its margins.
    let bounds = tree.root().abs_bounding_box();
    let scale = size as f32 / bounds.width().max(bounds.height());
    let dx = (size as f32 - bounds.width() * scale) / 2. - bounds.x() * scale;
    let dy = (size as f32 - bounds.height() * scale) / 2. - bounds.y() * scale;
    resvg::render(
        tree,
        resvg::tiny_skia::Transform::from_row(scale, 0., 0., scale, dx, dy),
        &mut pixmap.as_mut(),
    );

    let mask_stride = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    for value in [40, size, size * 2] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    // Icon DIBs are bottom-up BGRA; tiny-skia stores premultiplied RGBA.
    for row in pixmap.pixels().chunks_exact(size as usize).rev() {
        for pixel in row {
            let color = pixel.demultiply();
            out.extend_from_slice(&[color.blue(), color.green(), color.red(), color.alpha()]);
        }
    }
    out.resize(out.len() + (mask_stride * size) as usize, 0);
    out
}
