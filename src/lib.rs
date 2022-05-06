pub mod camera;
pub mod input_binding;
pub mod physics;
pub mod player;
pub mod util;
pub mod world;

use bevy_ecs_tilemap::Map;
use input_binding::InputBinder;
use player::spawn_player;
pub use player::{Player, PlayerProperties};
use world::GameWorld;
pub use world::LdtkProject;

use ggez::*;
use glam::*;

use ggez_egui::*;

use util::GameInstant;

use bevy::{asset::AssetServerSettings, prelude::*, render::camera::ScalingMode};

use crate::world::{LevelBundle, LevelId};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Playing,
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_bindings: ResMut<InputBinder>,
) {
    commands.spawn_bundle(OrthographicCameraBundle {
        orthographic_projection: OrthographicProjection {
            scale: 10.,
            scaling_mode: ScalingMode::FixedVertical,
            ..default()
        },
        ..OrthographicCameraBundle::new_2d()
    });

    commands.insert_resource(AssetServerSettings {
        watch_for_changes: true,
        ..default()
    });
    info!("Starting to load world file");
    let ldtk: Handle<LdtkProject> = asset_server.load("world.ldtk");
    spawn_player(&mut commands);
    commands.insert_resource(GameWorld { ldtk });

    let map_entity = commands.spawn().id();

    info!("Inserted level");
    commands.entity(map_entity).insert_bundle(LevelBundle {
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        level_id: LevelId("Level_0".to_owned()),
        ..Default::default()
    });

    // TODO: use asset loading for bindings
    input_bindings.load_from_str(include_str!("../assets/input.ron"));
}

pub struct Game {
    world: World,
    paused: bool,
    player_props_ui_visible: bool,
    camera_position: Vec2,
}

/*
impl MainState {
    pub fn new(ctx: &mut Context) -> GameResult<MainState> {
        let game_time = GameInstant::from_game_start();
        // FIXME: Wait until `GameResult` allows for any error instead of just `CustomError`
        let world = World::from_file(ctx, Path::new("/world/world.world")).unwrap();
        let current_level = ivec2(0, 0);
        let player = Player::new(
            ctx,
            //FIXME: spawnpoint is relative to room, need to make it absolute
            world.room(current_level).unwrap().level.spawn_point,
            game_time,
        )?;

        let read_input_bindings = || -> anyhow::Result<InputBinder> {
            let mut input_file = filesystem::open(ctx, "/input.ron")?;
            let mut input_binding_contents = String::new();
            input_file.read_to_string(&mut input_binding_contents)?;
            Ok(ron::from_str(&input_binding_contents)?)
        };

        let input_bindings = match read_input_bindings() {
            Ok(bindings) => bindings,
            Err(err) => {
                eprintln!("Couldn't load input bindings; Error: {}", err);
                Default::default()
            }
        };

        dbg!(input::gamepad::gamepads(ctx).collect::<Vec<_>>());

        Ok(MainState {
            player,
            world,
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
            input_bindings,
            player_props_ui_visible: false,
            camera_position: Vec2::ZERO,
        })
    }

    fn player_room_pos(&self) -> IVec2 {
        self.world.tile_to_room_pos(self.player.position())
    }
}

// FIXME: Wait for https://github.com/ggez/ggez/pull/1022 to have E = anyhow::Error
impl event::EventHandler<GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if !self.paused {
            self.player
                .update(ctx, &self.world, self.game_time, &self.input_bindings);
            self.game_time.add_unpaused_delta(timer::delta(ctx));
        }

        let egui_ctx = self.egui_backend.ctx();

        if self.player_props_ui_visible {
            self.player.properties.show_ui(&egui_ctx);
        }

        if self.world.room(self.player_room_pos()).is_none() {
            self.player
                .teleport_to(self.world.room(ivec2(0, 0)).unwrap().level.spawn_point);
        }

        self.input_bindings.finish_frame();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());
        let player_room_pos = self.player_room_pos();
        self.camera_position = (player_room_pos * self.world.room_size()).as_vec2();
        graphics::set_screen_coordinates(
            ctx,
            graphics::Rect {
                x: self.camera_position.x,
                y: self.camera_position.y,
                w: self.world.room_size().x as f32,
                h: self.world.room_size().y as f32,
            },
        )?;

        let player_room = self.world.room(player_room_pos).unwrap();
        player_room.level.draw(
            ctx,
            graphics::DrawParam::default()
                .dest((player_room.position * self.world.room_size()).as_vec2()),
        )?;
        self.player.draw(ctx, graphics::DrawParam::default())?;

        graphics::set_screen_coordinates(
            ctx,
            graphics::Rect {
                x: 0.,
                y: 0.,
                w: graphics::window(ctx).inner_size().width as f32,
                h: graphics::window(ctx).inner_size().height as f32,
            },
        )?;

        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "pos: {}, vel: {}",
                self.player.position(),
                self.player.velocity()
            )),
            graphics::DrawParam::default(),
        )?;

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
        println!("Down btn {:?}", btn);
        self.input_bindings.gamepad_button_down_event(btn, id)
    }

    fn gamepad_button_up_event(
        &mut self,
        _ctx: &mut Context,
        btn: event::Button,
        id: event::GamepadId,
    ) {
        println!("Up btn {:?}", btn);
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

        if keycode == event::KeyCode::R {
            self.player.teleport_to(
                self.world
                    .room(self.world.tile_to_room_pos(self.player.position()))
                    .unwrap()
                    .level
                    .spawn_point,
            );
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
*/
