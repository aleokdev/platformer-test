use ggez::*;
use platformer_test::MainState;

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("Platformer Template", "aleok")
        .window_setup(conf::WindowSetup::default().title("Platformer test"))
        .window_mode(conf::WindowMode::default().dimensions(40. * 18., 25. * 18.))
        .add_resource_path(
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("assets"),
        );
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
