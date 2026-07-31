//! Binary entry point.
//!
//! Subcommands grow with the milestones. Available now:
//!   verify-rom [path]                       check the ROM is SML (World) v1.0
//!   extract-tiles <offset> <count> <out>    decode ROM tiles into an asset file
//!   extract-level [1-1|1-2|1-3] [out]       decode a level into a level file
//!                 [--expert]                include the expert-mode objects
//!   scan-levels                             list every screen list in the ROM
//!   list-objects [1-1|1-2|1-3]              print a level's object list
//!   render-level <level> <col> <out.png>    draw a level screen with its real tiles
//!   screenshot <out.png>                    render a frame to a PNG (headless)

use std::process::ExitCode;

use sml::render::{render_background, Framebuffer, Palette};

#[cfg(feature = "gui")]
mod audio;

const DEFAULT_ROM: &str = "super_mario_land.gb";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify-rom") => {
            let path = args.get(1).map(String::as_str).unwrap_or(DEFAULT_ROM);
            verify_rom(path)
        }
        Some("extract-tiles") => extract_tiles(&args[1..]),
        Some("extract-title") => extract_title_screen(&args[1..]),
        Some("extract-level") => extract_level(&args[1..]),
        Some("scan-levels") => scan_levels(),
        Some("list-objects") => list_objects(&args[1..]),
        Some("render-level") => render_level(&args[1..]),
        Some("screenshot") => screenshot(&args[1..]),
        Some("render-title") => render_title(&args[1..]),
        Some("run") => run_game(&args[1..]),
        Some("play") => play(&args[1..]),
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

/// Opt back in to content Super Mario Land does not have (the star, the Fly).
/// The default build plays the cartridge's own content only; see
/// `docs/reference/faithfulness.md`.
const ALLOW_NON_CANONICAL: &str = "--allow-non-canonical";

/// Pull the opt-in flag out of the argument list, so the positional arguments
/// each command parses are unaffected by where the flag was written.
fn split_non_canonical_flag(args: &[String]) -> (Vec<String>, bool) {
    let allow = args.iter().any(|a| a == ALLOW_NON_CANONICAL);
    let rest = args
        .iter()
        .filter(|a| a.as_str() != ALLOW_NON_CANONICAL)
        .cloned()
        .collect();
    (rest, allow)
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

/// `extract-title [outdir]` reads the verified ROM and extracts the title
/// screen tiles and tilemap directly from known ROM offsets. No emulator
/// involved: tile data is read from the ROM binary at addresses pinned from
/// the kaspermeerts/supermarioland disassembly.
///
/// Output goes to `outdir` (default `assets/extracted/`):
///   title.tiles   our SMLT tile-sheet format (deduplicated tiles + palette)
///   title.tmap    our SMLM tile-map format (20x18 indices into the sheet)
fn extract_title_screen(args: &[String]) -> ExitCode {
    let outdir = args.first().map(String::as_str).unwrap_or("assets/extracted");
    let out_path = std::path::Path::new(outdir);
    if let Err(e) = std::fs::create_dir_all(out_path) {
        eprintln!("could not create output directory {outdir}: {e}");
        return ExitCode::FAILURE;
    }

    let (sheet, cells, cols, rows) = match sml::assets::title::extract_title(DEFAULT_ROM) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("title extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let tiles_path = out_path.join("title.tiles");
    if let Err(e) = sheet.save(&tiles_path) {
        eprintln!("could not write {}: {e}", tiles_path.display());
        return ExitCode::FAILURE;
    }

    // SMLM tilemap: magic, version, u16 width, u16 height, index bytes.
    let mut map_blob = b"SMLM".to_vec();
    map_blob.push(1); // version
    map_blob.extend_from_slice(&(cols as u16).to_le_bytes());
    map_blob.extend_from_slice(&(rows as u16).to_le_bytes());
    map_blob.extend_from_slice(&cells);
    let map_path = out_path.join("title.tmap");
    if let Err(e) = std::fs::write(&map_path, &map_blob) {
        eprintln!("could not write {}: {e}", map_path.display());
        return ExitCode::FAILURE;
    }

    // PGM preview of the tile sheet.
    let pgm_path = out_path.join("title.tiles.pgm");
    if let Err(e) = std::fs::write(&pgm_path, sheet.to_pgm(16)) {
        eprintln!("could not write {}: {e}", pgm_path.display());
        return ExitCode::FAILURE;
    }

    println!("extracted title screen from ROM (no emulator):");
    println!("  {} unique tiles -> {}", sheet.tiles.len(), tiles_path.display());
    println!("  {}x{} tilemap   -> {}", cols, rows, map_path.display());
    println!("  tile preview    -> {}", pgm_path.display());
    ExitCode::SUCCESS
}

/// `extract-level [1-1|1-2|1-3] [out]` decodes a level's geometry straight out of
/// the verified ROM and writes it in our plain-text level format, which
/// `Level::from_file` loads. No emulator involved.
///
/// Output defaults to `assets/extracted/level_<name>.txt`, gitignored and
/// regenerated on demand.
fn extract_level(args: &[String]) -> ExitCode {
    use sml::assets::{level, object};

    let expert = args.iter().any(|a| a == "--expert");
    let args: Vec<&String> = args.iter().filter(|a| *a != "--expert").collect();
    let mode = if expert {
        object::Mode::Expert
    } else {
        object::Mode::Normal
    };

    let name = args.first().map(|s| s.as_str()).unwrap_or("1-1");
    let Some(&(_, list)) = level::WORLD_1.iter().find(|(n, _)| *n == name) else {
        let known: Vec<&str> = level::WORLD_1.iter().map(|(n, _)| *n).collect();
        eprintln!("unknown level {name}; known levels are {}", known.join(", "));
        return ExitCode::FAILURE;
    };
    let suffix = if expert { "_expert" } else { "" };
    let default_out = format!(
        "assets/extracted/level_{}{suffix}.txt",
        name.replace('-', "_")
    );
    let out = args.get(1).map(|s| s.as_str()).unwrap_or(&default_out);
    let path = std::path::Path::new(out);
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("could not create output directory {}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    }

    let columns = match level::extract_level(DEFAULT_ROM, list) {
        Ok(columns) => columns,
        Err(e) => {
            eprintln!("level extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let objects = object::WORLD_1_OBJECTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, start)| object::extract_objects(DEFAULT_ROM, start))
        .transpose();
    let records = match objects {
        Ok(records) => records.unwrap_or_default(),
        Err(e) => {
            eprintln!("object extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let walkers = object::walker_spawns(&records, mode);
    let lifts = object::lift_spawns(&records, mode);
    let hoppers = object::hopper_spawns(&records, mode);
    let mut objects: Vec<(usize, usize, u8)> =
        walkers
            .iter()
            .map(|&(c, r, turns)| (c, r, if turns { b'T' } else { b'G' }))
            .collect();
    objects.extend(
        lifts
            .iter()
            .map(|&(c, r, up)| (c, r, if up { b'V' } else { b'H' })),
    );
    objects.extend(hoppers.iter().map(|&(c, r)| (c, r, b'J')));

    let text = level::to_level_text_with_objects(&columns, &objects);
    if let Err(e) = std::fs::write(path, text) {
        eprintln!("could not write {out}: {e}");
        return ExitCode::FAILURE;
    }

    let solid = columns
        .iter()
        .flatten()
        .filter(|&&t| level::is_solid(t))
        .count();
    let platforms = columns
        .iter()
        .flatten()
        .filter(|&&t| level::is_platform(t))
        .count();
    let coins = columns
        .iter()
        .flatten()
        .filter(|&&t| level::is_coin(t))
        .count();
    println!("extracted World {name} from ROM (no emulator):");
    println!("  {}x{} -> {out}", columns.len(), level::ROWS);
    println!("  {solid} solid cells, {platforms} platform cells, {coins} coins");
    println!(
        "  {} ground walkers, {} jumpers and {} lifts, from {} object records \
({} spawn in {} mode)",
        walkers.len(),
        hoppers.len(),
        lifts.len(),
        records.len(),
        object::spawning_in(&records, mode).len(),
        if expert { "expert" } else { "normal" },
    );
    println!("  columns you can fall through: {:?}", level::pits(&columns));
    ExitCode::SUCCESS
}

/// `render-level [1-1|1-2|1-3] <column> <out.png>` draws one screen of a level
/// with the cartridge's own tile graphics, straight from the ROM.
fn render_level(args: &[String]) -> ExitCode {
    use sml::assets::level;
    use sml::render::{Framebuffer, Palette, TileMap};

    let [name, column, out] = args else {
        eprintln!("usage: sml render-level <1-1|1-2|1-3> <column> <out.png>");
        return ExitCode::FAILURE;
    };
    let Some(&(_, list)) = level::WORLD_1.iter().find(|(n, _)| n == name) else {
        eprintln!("unknown level {name}");
        return ExitCode::FAILURE;
    };
    let Ok(column) = column.parse::<usize>() else {
        eprintln!("column must be a number");
        return ExitCode::FAILURE;
    };

    let (sheet, cells) = match level::extract_screen(DEFAULT_ROM, list, column) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("screen extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let map = TileMap::new(level::SCREEN_COLUMNS, level::SCREEN_ROWS, cells);
    let mut fb = Framebuffer::new();
    sml::render::render_background(&mut fb, &map, &sheet.tiles, 0, 0, &Palette::new(sheet.palette));
    let png = sml::png::encode_gray(160, 144, &fb.to_gray());
    if let Err(e) = std::fs::write(out, png) {
        eprintln!("could not write {out}: {e}");
        return ExitCode::FAILURE;
    }
    println!("drew World {name} from column {column} ({} unique tiles) -> {out}", sheet.tiles.len());
    ExitCode::SUCCESS
}

/// `list-objects [1-1]` prints a level's object list: where each record puts
/// its object in the level, and whether normal play acts on it at all.
fn list_objects(args: &[String]) -> ExitCode {
    use sml::assets::object;

    let name = args.first().map(String::as_str).unwrap_or("1-1");
    let Some(&(_, start)) = object::WORLD_1_OBJECTS.iter().find(|(n, _)| *n == name) else {
        let known: Vec<&str> = object::WORLD_1_OBJECTS.iter().map(|(n, _)| *n).collect();
        eprintln!("unknown level {name}; known levels are {}", known.join(", "));
        return ExitCode::FAILURE;
    };
    let records = match object::extract_objects(DEFAULT_ROM, start) {
        Ok(records) => records,
        Err(e) => {
            eprintln!("object extraction failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("World {name}: {} records at 0x{start:05X}", records.len());
    println!(" bytes     pixel x  column  row  kind  mode");
    for r in &records {
        println!(
            " {:02X} {:02X} {:02X}   {:7}  {:6}  {:3}  0x{:02X}  {}",
            r.x,
            r.y,
            r.kind,
            r.pixel_x(),
            r.column(),
            r.row(),
            r.kind_id(),
            if r.expert_only() { "expert" } else { "both" },
        );
    }
    println!(
        "{} spawn in normal play, all {} in expert mode",
        object::spawning(&records).len(),
        records.len(),
    );
    ExitCode::SUCCESS
}

/// `scan-levels` finds every screen list in the ROM by structure. Nothing in
/// the ROM holds World 1-1's list address, so there is no table to follow.
fn scan_levels() -> ExitCode {
    use sml::assets::level;

    let rom = match std::fs::read(DEFAULT_ROM) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("could not read {DEFAULT_ROM}: {e}");
            return ExitCode::FAILURE;
        }
    };
    for (start, pointers) in level::find_screen_lists(&rom) {
        let unique: std::collections::BTreeSet<u16> = pointers.iter().copied().collect();
        let mark = if start <= level::LEVEL_1_1_LIST
            && level::LEVEL_1_1_LIST < start + 2 * pointers.len()
        {
            "  <- contains World 1-1's list"
        } else {
            ""
        };
        println!(
            "0x{start:05X}  {:2} screens ({} unique), {} columns{mark}",
            pointers.len(),
            unique.len(),
            pointers.len() * level::SCREEN_COLUMNS
        );
        let hex: Vec<String> = pointers.iter().map(|p| format!("{p:04X}")).collect();
        println!("          {}", hex.join(" "));
    }
    ExitCode::SUCCESS
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
/// so run `sml extract-title` first to produce them.
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
            eprintln!("run: cargo run -- extract-title");
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

/// `play <out.png> [frames] [keys]` runs the game headlessly with a fixed set of
/// buttons held for `frames` frames, then writes the final frame as a PNG. Keys
/// are plus-separated: left,right,up,down,a,b,start,select (e.g. right+a). This
/// is how gameplay gets inspected and captured without ever opening a window.
fn play(args: &[String]) -> ExitCode {
    use sml::core::level::Level;
    use sml::game::Game;
    use sml::input::{Button, Buttons};
    use sml::tuning::Tuning;

    let (args, allow_non_canonical) = split_non_canonical_flag(args);
    let args = args.as_slice();
    let (out, frames, keys, level, tuning_path) = match args {
        [out] => (out.as_str(), 1u32, "", None, None),
        [out, frames] => (out.as_str(), frames.parse().unwrap_or(1), "", None, None),
        [out, frames, keys] => (out.as_str(), frames.parse().unwrap_or(1), keys.as_str(), None, None),
        [out, frames, keys, level] => (
            out.as_str(),
            frames.parse().unwrap_or(1),
            keys.as_str(),
            Some(level.as_str()),
            None,
        ),
        [out, frames, keys, level, tuning] => (
            out.as_str(),
            frames.parse().unwrap_or(1),
            keys.as_str(),
            Some(level.as_str()),
            Some(tuning.as_str()),
        ),
        _ => {
            eprintln!("usage: sml play <out.png> [frames] [keys] [level.txt] [tuning.txt]");
            eprintln!("  keys: plus-separated, e.g. right+a");
            return ExitCode::FAILURE;
        }
    };

    // "big" is not a button: it starts Mario in his big state, so any size can
    // be captured headlessly.
    let mut buttons = Buttons::default();
    let mut start_big = false;
    let mut start_fire = false;
    for name in keys.split('+').filter(|s| !s.is_empty()) {
        let button = match name.to_lowercase().as_str() {
            "left" => Button::Left,
            "right" => Button::Right,
            "up" => Button::Up,
            "down" => Button::Down,
            "a" => Button::A,
            "b" => Button::B,
            "start" => Button::Start,
            "select" => Button::Select,
            "big" => {
                start_big = true;
                continue;
            }
            "fire" => {
                start_fire = true;
                continue;
            }
            other => {
                eprintln!("unknown key: {other}");
                return ExitCode::FAILURE;
            }
        };
        buttons.set(button, true);
    }

    let level = match level {
        Some(path) => match Level::from_file(path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("could not load level {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Game::demo_level(),
    };
    let level = if allow_non_canonical {
        level
    } else {
        level.without_non_canonical()
    };
    let mut game = Game::new(level);
    if let Some(path) = tuning_path {
        match std::fs::read_to_string(path) {
            Ok(text) => match Tuning::from_text(&text) {
                Ok(t) => game.tuning = t,
                Err(e) => {
                    eprintln!("bad tuning file {path}: {e}");
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                eprintln!("could not read tuning file {path}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    if start_big || start_fire {
        game.grow_mario();
    }
    if start_fire {
        game.mario.power = sml::core::entity::Power::Fire;
    }
    for _ in 0..frames {
        game.step(buttons);
    }
    let png = sml::png::encode_gray(sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT, &game.render().to_gray());
    match std::fs::write(out, png) {
        Ok(()) => {
            println!("played {frames} frames (held: {keys:?}) -> {out}");
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
fn run_game(args: &[String]) -> ExitCode {
    use minifb::{Key, Window, WindowOptions};
    use sml::core::level::Level;
    use sml::session::default_levels;
    use sml::input::mapping::{buttons_from_held, Key as GbKey};
    use sml::session::Session;
    use sml::tuning::Tuning;

    const SCALE: usize = 4;

    // The whole game lives in Session (headless and tested): title, play, and
    // the end screens. This is just a shell that feeds it keys and blits frames.
    // An optional level file plays a custom level instead of the built-in demo,
    // and an optional tuning file (only meaningful alongside a level) retunes
    // the physics without a recompile (see docs/reference/level-format.md).
    let (args, allow_non_canonical) = split_non_canonical_flag(args);
    let level_path = args.first();
    let tuning_path = args.get(1);

    let tuning = match tuning_path {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => match Tuning::from_text(&text) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("bad tuning file {path}: {e}");
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                eprintln!("could not read tuning file {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Tuning::default(),
    };

    let faithful = |levels: Vec<Level>| -> Vec<Level> {
        if allow_non_canonical {
            levels
        } else {
            levels.into_iter().map(Level::without_non_canonical).collect()
        }
    };
    let mut session = match level_path {
        Some(path) => match Level::from_file(path) {
            Ok(level) => Session::with_tuning(faithful(vec![level]), tuning),
            Err(e) => {
                eprintln!("could not load level {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Session::with_tuning(faithful(default_levels()), tuning),
    };

    let (win_w, win_h) = sml::frontend::scaled_size(SCALE);
    let mut window = match Window::new("Super Mario Land in Rust", win_w, win_h, WindowOptions::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("could not open window: {e}");
            return ExitCode::FAILURE;
        }
    };
    window.set_target_fps(60);

    let player = audio::AudioPlayer::try_new();
    if player.is_none() {
        eprintln!("no audio output device found; running muted");
    }

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut held = Vec::new();
        if window.is_key_down(Key::Left) { held.push(GbKey::Left); }
        if window.is_key_down(Key::Right) { held.push(GbKey::Right); }
        if window.is_key_down(Key::Up) { held.push(GbKey::Up); }
        if window.is_key_down(Key::Down) { held.push(GbKey::Down); }
        if window.is_key_down(Key::Z) { held.push(GbKey::Z); }
        if window.is_key_down(Key::X) { held.push(GbKey::X); }
        if window.is_key_down(Key::Enter) { held.push(GbKey::Enter); }

        session.step(buttons_from_held(held));
        if let Some(player) = &player {
            for event in session.drain_sounds() {
                player.play(sml::frontend::tone_for(event));
            }
        } else {
            session.drain_sounds();
        }

        let argb = sml::frontend::to_argb(&session.render(), SCALE);
        if let Err(e) = window.update_with_buffer(&argb, win_w, win_h) {
            eprintln!("window update failed: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "gui"))]
fn run_game(_args: &[String]) -> ExitCode {
    eprintln!("this build has no window support.");
    eprintln!("rebuild with the gui feature: cargo run --features gui -- run [level.txt]");
    ExitCode::FAILURE
}

fn usage() {
    println!("\nusage:");
    println!("  sml                                       print status");
    println!("  sml verify-rom [path]                     verify the ROM hashes");
    println!("  sml extract-tiles <offset> <count> <out>  decode ROM tiles to an asset file");
    println!("  sml extract-title [outdir]                extract title screen from ROM");
    println!("  sml screenshot <out.png>                  render a frame to a PNG");
    println!("  sml render-title <out.png>                render the extracted title screen");
    println!("  sml run [level.txt] [tuning.txt]          play in a window (needs --features gui)");
    println!("  sml play <out.png> [frames] [keys] [lvl] [tuning]  run headlessly to a PNG");
    println!("\n  run and play accept --allow-non-canonical anywhere in the arguments,");
    println!("  which keeps content Super Mario Land does not have (the star, the Fly).");
    println!("  Without it those spawns are dropped, so the default game is the cartridge's.");
}
