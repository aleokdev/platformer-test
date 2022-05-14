use bevy::prelude::*;

#[derive(Default)]
pub struct DebugMode {
    pub active: bool,
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>();
        #[cfg(debug_assertions)]
        app.add_system(debug_mode_activator);
    }
}

#[cfg(debug_assertions)]
fn debug_mode_activator(input: Res<Input<KeyCode>>, mut debug: ResMut<DebugMode>) {
    if input.just_pressed(KeyCode::I) {
        debug.active = !debug.active;
    }
}
