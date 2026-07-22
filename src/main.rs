//! Binary entry point.
//!
//! Real subcommands (run, screenshot, verify-rom) arrive with their milestones.
//! For now this confirms the workspace builds and runs.

fn main() {
    println!(
        "Super Mario Land in Rust: workspace bootstrap. Target screen {}x{}.",
        sml::SCREEN_WIDTH,
        sml::SCREEN_HEIGHT
    );
    println!("See docs/GRAND_MASTER_PLAN.md for what happens next.");
}
