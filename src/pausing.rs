use bevy::prelude::*;

use crate::{
    input_mapper::{self, Action, ActionState},
    AppState,
};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(pause_on_esc);
    }
}

fn pause_on_esc(input: Res<input_mapper::Input>, mut state: ResMut<State<AppState>>) {
    if input.actions[Action::Pause] == ActionState::JustPressed {
        if state.current() == &AppState::Playing {
            state.set(AppState::Paused).unwrap();
        } else if state.current() == &AppState::Paused {
            state.set(AppState::Playing).unwrap();
        }
    }
}
