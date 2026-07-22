//! Binary entry point.
//!
//! Subcommands grow with the milestones. Available now:
//!   verify-rom [path]                       check the ROM is SML (World) v1.0
//!   extract-tiles <offset> <count> <out>    decode ROM tiles into an asset file
//!   screenshot <out.png>                    render a frame to a PNG (headless)

use std::process::ExitCode;

use sml::render::{render_background, Framebuffer, Palette};

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
        Some("render-title") => render_title(&args[1..]),
        Some("run") => run_game(),
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

    let fb = sml::scene::demo();
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

/// `render-title <out.png>` loads the extracted title-screen assets and renders
/// them through our own pipeline to a PNG. The extracted assets are gitignored,
/// so run `uv run tools/extract_title.py` first to produce them.
fn render_title(args: &[String]) -> ExitCode {
    let out = match args {
        [out] => out,
        _ => {
            eprintln!("usage: sml render-title <out.png>");
            return ExitCode::FAILURE;
        }
    };

    let sheet = match sml::assets::TileSheet::load("assets/extracted/title.tiles") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not load title tiles: {e}");
            eprintln!("run: uv run tools/extract_title.py");
            return ExitCode::FAILURE;
        }
    };
    let map = match sml::assets::load_tilemap("assets/extracted/title.tmap") {
        Ok(m) => m,
        Err(e) => {
            eprintln!("could not load title map: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut fb = Framebuffer::new();
    render_background(&mut fb, &map, &sheet.tiles, 0, 0, &Palette::new(sheet.palette));
    let png = sml::png::encode_gray(sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT, &fb.to_gray());
    match std::fs::write(out, png) {
        Ok(()) => {
            println!("rendered title screen to {out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("could not write {out}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Open a window and play a small test level: Mario on solid ground with a
/// scrolling camera, driven by the keyboard. Only built with `--features gui`.
#[cfg(feature = "gui")]
fn run_game() -> ExitCode {
    use minifb::{Key, Window, WindowOptions};
    use sml::camera::Camera;
    use sml::core::level::{Level, TILE};
    use sml::core::physics::step_motion;
    use sml::input::mapping::{buttons_from_held, Key as GbKey};
    use sml::render::{render_background, TileMap};
    use sml::tiles::Tile;

    const SCALE: usize = 4;

    let solid = |i: u8| Tile { pixels: [[i; 8]; 8] };

    // Build a wide test level: a floor, a couple of platforms, Mario near the left.
    let (w, h) = (40usize, 18usize);
    let mut rows: Vec<String> = Vec::new();
    for y in 0..h {
        let mut row = String::new();
        for x in 0..w {
            let floor = y >= h - 2;
            let platform =
                (y == h - 5 && (10..14).contains(&x)) || (y == h - 8 && (20..26).contains(&x));
            let c = if floor || platform {
                '#'
            } else if x == 2 && y == h - 3 {
                'M'
            } else {
                '.'
            };
            row.push(c);
        }
        rows.push(row);
    }
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let level = Level::from_rows(&refs);

    // Visuals: empty tiles white, solid tiles dark; Mario a black block.
    let bg_tiles = [solid(0), solid(2)];
    let mut bg_cells = Vec::with_capacity(w * h);
    for ty in 0..h {
        for tx in 0..w {
            bg_cells.push(if level.solids.is_solid(tx as i32, ty as i32) { 1 } else { 0 });
        }
    }
    let bg_map = TileMap::new(w, h, bg_cells);
    let mario_tile = solid(3);
    let palette = Palette::new(0xE4);

    let level_w = (w as i32) * TILE;
    let level_h = (h as i32) * TILE;

    let mut mario = sml::core::entity::Mario::new(level.spawn.0, level.spawn.1);
    let mut camera = Camera::new();

    let (win_w, win_h) = sml::frontend::scaled_size(SCALE);
    let mut window = match Window::new("Super Mario Land in Rust", win_w, win_h, WindowOptions::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("could not open window: {e}");
            return ExitCode::FAILURE;
        }
    };
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut held = Vec::new();
        if window.is_key_down(Key::Left) { held.push(GbKey::Left); }
        if window.is_key_down(Key::Right) { held.push(GbKey::Right); }
        if window.is_key_down(Key::Up) { held.push(GbKey::Up); }
        if window.is_key_down(Key::Down) { held.push(GbKey::Down); }
        if window.is_key_down(Key::Z) { held.push(GbKey::Z); }
        if window.is_key_down(Key::X) { held.push(GbKey::X); }
        if window.is_key_down(Key::Enter) { held.push(GbKey::Enter); }
        let buttons = buttons_from_held(held);

        step_motion(&mut mario, buttons, &level.solids);
        camera.follow(mario.pixel_x() + 4, mario.pixel_y() + 4, level_w, level_h);

        let mut fb = Framebuffer::new();
        render_background(&mut fb, &bg_map, &bg_tiles, camera.x, camera.y, &palette);
        fb.draw_tile(&mario_tile, mario.pixel_x() - camera.x, mario.pixel_y() - camera.y, &palette);

        let argb = sml::frontend::to_argb(&fb, SCALE);
        if let Err(e) = window.update_with_buffer(&argb, win_w, win_h) {
            eprintln!("window update failed: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "gui"))]
fn run_game() -> ExitCode {
    eprintln!("this build has no window support.");
    eprintln!("rebuild with the gui feature: cargo run --features gui -- run");
    ExitCode::FAILURE
}

fn usage() {
    println!("\nusage:");
    println!("  sml                                       print status");
    println!("  sml verify-rom [path]                     verify the ROM hashes");
    println!("  sml extract-tiles <offset> <count> <out>  decode ROM tiles to an asset file");
    println!("  sml screenshot <out.png>                  render a frame to a PNG");
    println!("  sml render-title <out.png>                render the extracted title screen");
    println!("  sml run                                   play in a window (needs --features gui)");
}
