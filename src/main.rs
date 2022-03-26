use ggez::*;
use platformer_test::MainState;

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("super_simple", "aleok")
        .window_setup(conf::WindowSetup::default().title("Platformer test"))
        .add_resource_path(
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("assets"),
        );
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
