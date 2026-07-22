//! Binary entry point.
//!
//! Subcommands grow with the milestones. Available now:
//!   verify-rom [path]                       check the ROM is SML (World) v1.0
//!   extract-tiles <offset> <count> <out>    decode ROM tiles into an asset file
//!   screenshot <out.png>                    render a frame to a PNG (headless)

use std::process::ExitCode;

use sml::render::{render_background, Framebuffer, Palette, TileMap};
use sml::tiles::Tile;

const DEFAULT_ROM: &str = "super_mario_land.gb";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify-rom") => {
            let path = args.get(1).map(String::as_str).unwrap_or(DEFAULT_ROM);
            verify_rom(path)
        }
        Some("extract-tiles") => extract_tiles(&args[1..]),
        Some("screenshot") => screenshot(&args[1..]),
        Some(other) => {
            eprintln!("unknown command: {other}");
            usage();
            ExitCode::FAILURE
        }
        None => {
            println!(
                "Super Mario Land in Rust: workspace bootstrap. Target screen {}x{}.",
                sml::SCREEN_WIDTH,
                sml::SCREEN_HEIGHT
            );
            usage();
            ExitCode::SUCCESS
        }
    }
}

fn verify_rom(path: &str) -> ExitCode {
    match sml::rom::verify_file(path) {
        Ok(hashes) => {
            println!("ROM verified as Super Mario Land (World) v1.0:\n{hashes}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// `extract-tiles <offset> <count> <out.tiles>` where offset is hex (0x...) or
/// decimal. Writes our asset format, and a .pgm preview beside it.
fn extract_tiles(args: &[String]) -> ExitCode {
    let (offset, count, out) = match args {
        [offset, count, out] => (offset, count, out),
        _ => {
            eprintln!("usage: sml extract-tiles <offset> <count> <out.tiles>");
            return ExitCode::FAILURE;
        }
    };

    let offset = match parse_number(offset) {
        Some(n) => n,
        None => {
            eprintln!("bad offset: {offset}");
            return ExitCode::FAILURE;
        }
    };
    let count = match parse_number(count) {
        Some(n) => n,
        None => {
            eprintln!("bad count: {count}");
            return ExitCode::FAILURE;
        }
    };

    let sheet = match sml::assets::extract_tiles(DEFAULT_ROM, offset, count) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = sheet.save(out) {
        eprintln!("could not write {out}: {e}");
        return ExitCode::FAILURE;
    }
    let preview = format!("{out}.pgm");
    if let Err(e) = std::fs::write(&preview, sheet.to_pgm(16)) {
        eprintln!("could not write preview {preview}: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "extracted {} tiles from offset {:#06x} -> {out} (preview {preview})",
        sheet.tiles.len(),
        offset
    );
    ExitCode::SUCCESS
}

fn parse_number(s: &str) -> Option<usize> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// `screenshot <out.png>` renders one frame headlessly and writes a PNG.
/// Until the title screen assets are extracted, this draws a demo scene so the
/// render-to-image path can be exercised and eyeballed.
fn screenshot(args: &[String]) -> ExitCode {
    let out = match args {
        [out] => out,
        _ => {
            eprintln!("usage: sml screenshot <out.png>");
            return ExitCode::FAILURE;
        }
    };

    let fb = demo_scene();
    let png = sml::png::encode_gray(sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT, &fb.to_gray());
    match std::fs::write(out, png) {
        Ok(()) => {
            println!("wrote {}x{} screenshot to {out}", sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("could not write {out}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// A placeholder frame: four solid shades arranged as diagonal bands, drawn
/// through the real tilemap path so the whole render pipeline is exercised.
fn demo_scene() -> Framebuffer {
    let tiles: Vec<Tile> = (0..4).map(|i| solid_tile(i as u8)).collect();
    let (w, h) = (20usize, 18usize);
    let cells = (0..w * h)
        .map(|c| (((c % w) + (c / w)) % 4) as u8)
        .collect();
    let map = TileMap::new(w, h, cells);
    let mut fb = Framebuffer::new();
    render_background(&mut fb, &map, &tiles, 0, 0, &Palette::new(0xE4));
    fb
}

fn solid_tile(color_index: u8) -> Tile {
    Tile {
        pixels: [[color_index; 8]; 8],
    }
}

fn usage() {
    println!("\nusage:");
    println!("  sml                                       print status");
    println!("  sml verify-rom [path]                     verify the ROM hashes");
    println!("  sml extract-tiles <offset> <count> <out>  decode ROM tiles to an asset file");
    println!("  sml screenshot <out.png>                  render a frame to a PNG");
}
