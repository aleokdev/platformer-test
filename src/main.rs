pub mod input_binding;
pub mod level;
pub mod player;
use std::{collections::HashMap, path::Path, time::Duration};

pub use level::{Level, LevelTile};
pub use player::{Player, PlayerProperties};

use ggez::*;
use glam::*;

use ggez_egui::*;

use path_clean::PathClean;
use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GameInstant {
    time_unpaused: Duration,
}

impl GameInstant {
    pub fn from_game_start() -> Self {
        Self {
            time_unpaused: Duration::ZERO,
        }
    }

    pub fn add_unpaused_delta(&mut self, delta: Duration) {
        self.time_unpaused += delta;
    }
}

impl std::ops::Add<Duration> for GameInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            time_unpaused: self.time_unpaused + rhs,
        }
    }
}

struct World {
    levels: HashMap<IVec2, Level>,
}

impl World {
    fn from_file(ctx: &mut Context, path: &Path) -> anyhow::Result<Self> {
        #[derive(Deserialize)]
        struct MapRefJson {
            #[serde(rename = "fileName")]
            filename: std::path::PathBuf,
            x: i32,
            y: i32,
        }
        #[derive(Deserialize)]
        struct WorldJson {
            maps: Vec<MapRefJson>,
        }
        let json: WorldJson = serde_json::from_reader(filesystem::open(ctx, path)?)?;
        let mut loader = tiled::Loader::with_cache_and_reader(
            tiled::DefaultResourceCache::new(),
            FsContext(ctx),
        );
        let dir = path.parent().unwrap();

        Ok(Self {
            levels: json
                .maps
                .into_iter()
                .map(|map| -> Result<_, anyhow::Error> {
                    let path = dir.join(map.filename).clean();
                    dbg!(&path);
                    Ok((
                        ivec2(map.x, map.y),
                        Level::new(loader.load_tmx_map(path)?, loader.reader_mut().0)?,
                    ))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn level(&self, pos: IVec2) -> &Level {
        &self.levels[&pos]
    }
}

struct MainState {
    world: World,
    current_level: IVec2,
    player: Player,
    egui_backend: EguiBackend,
    paused: bool,
    game_time: GameInstant,
    screen_rect_mesh: graphics::Mesh,
    paused_text: graphics::Text,
    input_bindings: input_binding::InputBinder,
    player_props_ui_visible: bool,
}

// Need to do newtype to implement ResourceReader for ggez's filesystem
// FIXME: This would greatly improve with separated subcontexts (ggez 0.8.0)
pub struct FsContext<'ctx>(pub &'ctx mut ggez::Context);

impl tiled::ResourceReader for FsContext<'_> {
    type Resource = filesystem::File;

    type Error = GameError;

    fn read_from(
        &mut self,
        path: &std::path::Path,
    ) -> std::result::Result<Self::Resource, Self::Error> {
        filesystem::open(&self.0, path)
    }
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let game_time = GameInstant::from_game_start();
        // FIXME: Wait until `GameResult` allows for actually any error instead of just `CustomError`
        let world = World::from_file(ctx, Path::new("/world/world.world")).unwrap();
        let current_level = ivec2(0, 0);
        let s = MainState {
            player: Player::new(ctx, world.levels[&current_level].spawn_point, game_time)?,
            world,
            current_level,
            egui_backend: EguiBackend::new(ctx),
            paused: false,
            game_time,
            screen_rect_mesh: graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::Fill(graphics::FillOptions::default()),
                graphics::Rect::new(0., 0., 1000000., 1000000.),
                graphics::Color::from_rgba(0, 0, 0, 80),
            )
            .unwrap(),
            paused_text: graphics::Text::new("Paused"),
            input_bindings: Default::default(),
            player_props_ui_visible: false,
        };
        Ok(s)
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if !self.paused {
            self.player.update(
                ctx,
                self.world.level(self.current_level),
                self.game_time,
                &self.input_bindings,
            );
            self.game_time.add_unpaused_delta(timer::delta(ctx));
        }

        let egui_ctx = self.egui_backend.ctx();

        if self.player_props_ui_visible {
            self.player.properties.show_ui(&egui_ctx);
        }

        self.input_bindings.finish_frame();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());

        self.world
            .level(self.current_level)
            .draw(ctx, graphics::DrawParam::default().scale([18., 18.]))?;
        self.player
            .draw(ctx, graphics::DrawParam::default().scale([18., 18.]))?;

        if self.paused {
            graphics::draw(ctx, &self.screen_rect_mesh, graphics::DrawParam::default())?;
            graphics::queue_text(ctx, &self.paused_text, vec2(20., 20.), None);
        }

        graphics::draw_queued_text(
            ctx,
            graphics::DrawParam::default(),
            None,
            graphics::FilterMode::Linear,
        )?;

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
        self.input_bindings.mouse_button_down_event(button);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: input::mouse::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_backend.input.mouse_button_up_event(button);
        self.input_bindings.mouse_button_up_event(button);
    }

    fn gamepad_button_down_event(
        &mut self,
        _ctx: &mut Context,
        btn: event::Button,
        id: event::GamepadId,
    ) {
        self.input_bindings.gamepad_button_down_event(btn, id)
    }

    fn gamepad_button_up_event(
        &mut self,
        _ctx: &mut Context,
        btn: event::Button,
        id: event::GamepadId,
    ) {
        self.input_bindings.gamepad_button_up_event(btn, id)
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.egui_backend.input.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.egui_backend.input.mouse_wheel_event(x, y);
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut Context,
        keycode: event::KeyCode,
        keymods: event::KeyMods,
    ) {
        self.input_bindings.key_up_event(keycode, keymods);
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: event::KeyCode,
        keymods: event::KeyMods,
        repeat: bool,
    ) {
        self.egui_backend.input.key_down_event(keycode, keymods);
        self.input_bindings.key_down_event(keycode, keymods, repeat);

        self.player
            .key_down_event(ctx, keycode, keymods, repeat, self.game_time);
        if keycode == event::KeyCode::R {
            self.player
                .teleport_to(self.world.level(self.current_level).spawn_point);
        }
        if keycode == event::KeyCode::Escape {
            self.paused = !self.paused;
        }
        if keycode == event::KeyCode::I && keymods == event::KeyMods::CTRL {
            self.player_props_ui_visible = !self.player_props_ui_visible;
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
        .add_resource_path(
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("assets"),
        );
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
