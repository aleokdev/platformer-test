pub mod level;
pub mod player;
pub use level::{Level, LevelTile};
pub use player::{Player, PlayerProperties};

use ggez::*;
use glam::*;

use ggez_egui::*;

struct MainState {
    level: Level,
    player: Player,
    egui_backend: EguiBackend,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let level = Level::new(
            tiled::Loader::new().load_tmx_map("assets/map.tmx").unwrap(),
            ctx,
        )?;
        let s = MainState {
            player: Player::new(ctx, level.spawn_point)?,
            level,
            egui_backend: EguiBackend::new(ctx),
        };
        Ok(s)
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.player.update(ctx, &self.level);

        let egui_ctx = self.egui_backend.ctx();

        self.player.properties.show_ui(&egui_ctx);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());

        self.level
            .draw(ctx, graphics::DrawParam::default().scale([18., 18.]))?;
        self.player
            .draw(ctx, graphics::DrawParam::default().scale([18., 18.]))?;

        graphics::draw(ctx, &self.egui_backend, ([0.0, 0.0],))?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: input::mouse::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_backend.input.mouse_button_down_event(button);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: input::mouse::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_backend.input.mouse_button_up_event(button);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.egui_backend.input.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.egui_backend.input.mouse_wheel_event(x, y);
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: event::KeyCode,
        keymods: event::KeyMods,
        repeat: bool,
    ) {
        self.egui_backend.input.key_down_event(keycode, keymods);

        self.player.key_down_event(ctx, keycode, keymods, repeat);
        if keycode == event::KeyCode::R {
            self.player.teleport_to(self.level.spawn_point);
        }
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) {
        self.egui_backend.input.text_input_event(character);
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        self.egui_backend.input.resize_event(width, height);
    }
}

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("super_simple", "aleok")
        .window_setup(conf::WindowSetup::default().title("Platformer test"))
        .add_resource_path(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
