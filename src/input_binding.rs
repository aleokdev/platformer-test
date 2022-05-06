use std::collections::HashSet;

use bevy::{
    input::{
        gamepad::{AxisSettings, ButtonSettings, GamepadSettings},
        keyboard::KeyboardInput,
        mouse::MouseButtonInput,
    },
    prelude::*,
    utils::HashMap,
};
use enum_map::{enum_map, Enum, EnumMap};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Deserialize)]
#[non_exhaustive]
pub enum Action {
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Deserialize)]
#[non_exhaustive]
pub enum Axis {
    Horizontal,
}

/// Triggers that have a state defined by an [ActionState] value.
#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DigitalTrigger {
    Key(KeyCode),
    MouseButton(MouseButton),
    GamepadButton(GamepadButton),
}

/// Triggers that have a state defined by a value in the `-1f32..1f32` range.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnalogTrigger {
    /// Emulates a real joystick axis with two [`DigitalTrigger`]s.
    ///
    /// - If `negative` is pressed, then its state will be -1.
    /// - If `positive` is pressed, then its state will be 1.
    /// - If both or none are pressed, then its state will be 0.
    DigitalJoystick {
        negative: DigitalTrigger,
        positive: DigitalTrigger,
    },
    GamepadAxis(GamepadAxis),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum ActionState {
    Released,
    JustReleased,
    JustPressed,
    Held,
}

impl ActionState {
    pub fn is_pressed(self) -> bool {
        matches!(self, Self::JustPressed | Self::Held)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct AxisState(f32);

impl AxisState {
    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug)]
struct TriggerRecord {
    just_pressed: HashSet<DigitalTrigger>,
    held: HashSet<DigitalTrigger>,
    just_released: HashSet<DigitalTrigger>,

    axis_values: HashMap<GamepadAxis, AxisState>,
}

impl TriggerRecord {
    fn new() -> Self {
        Self {
            just_pressed: HashSet::new(),
            held: HashSet::new(),
            just_released: HashSet::new(),

            axis_values: Default::default(),
        }
    }

    fn update_gamepad_button(&mut self, b: GamepadButton, state: f32, settings: &ButtonSettings) {
        let trigger = DigitalTrigger::GamepadButton(b);
        match state {
            state if state > settings.press => {
                // Pressed
                if !self.held.contains(&trigger) {
                    self.press(trigger);
                }
            }
            state if state < settings.release => {
                // Released
                if self.held.contains(&trigger) {
                    self.release(trigger);
                }
            }
            _ => (),
        };
    }

    fn update_gamepad_axis(&mut self, a: GamepadAxis, state: f32, settings: &AxisSettings) {
        let val = match state {
            state if state > settings.positive_high => 1.0,
            state if state < settings.negative_high => -1.0,
            state if state > settings.negative_low && state < settings.positive_low => 0.0,
            // Threshold shouldn't be our problem
            _ => state,
        };
        self.axis_values.insert(a, AxisState(val));
    }

    fn press(&mut self, trigger: DigitalTrigger) {
        if !self.held.contains(&trigger) {
            self.just_pressed.insert(trigger);
        }
    }

    fn release(&mut self, trigger: DigitalTrigger) {
        self.held.retain(|x| match (x, &trigger) {
            (DigitalTrigger::Key(l_key), DigitalTrigger::Key(r_key)) => l_key != r_key,
            _ => x != &trigger,
        });
        self.just_released.insert(trigger);
    }

    /// Convert "just pressed" triggers to held ones and clear "just released" triggers
    fn finish_frame(&mut self) {
        self.held.extend(std::mem::take(&mut self.just_pressed));
        self.just_released.clear();
    }

    fn state(&self, trigger: &DigitalTrigger) -> ActionState {
        if self.held.contains(trigger) {
            ActionState::Held
        } else if self.just_pressed.contains(trigger) {
            ActionState::JustPressed
        } else if self.just_released.contains(trigger) {
            ActionState::JustReleased
        } else {
            ActionState::Released
        }
    }
}

impl Default for TriggerRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// A binding from an input source to an action.
#[derive(Deserialize, Debug)]
pub struct ActionBinding {
    primary: DigitalTrigger,
    secondary: Option<DigitalTrigger>,
}

impl ActionBinding {
    pub fn new(primary: DigitalTrigger, secondary: Option<DigitalTrigger>) -> Self {
        Self { primary, secondary }
    }
}

/// A binding from an input source to an axis.
#[derive(Deserialize, Debug)]
pub struct AxisBinding {
    primary: AnalogTrigger,
    secondary: Option<AnalogTrigger>,
}

impl AxisBinding {
    pub fn new(primary: AnalogTrigger, secondary: Option<AnalogTrigger>) -> Self {
        Self { primary, secondary }
    }
}

#[derive(Deserialize, Debug)]
pub struct InputBinder {
    actions: EnumMap<Action, ActionBinding>,
    axes: EnumMap<Axis, AxisBinding>,

    #[serde(skip_deserializing)]
    trigger_record: TriggerRecord,
}

impl Default for InputBinder {
    fn default() -> Self {
        Self {
            actions: enum_map! {
                Action::Jump => ActionBinding::new(
                        DigitalTrigger::Key(KeyCode::Space), Some(DigitalTrigger::GamepadButton(GamepadButton(Gamepad(0), GamepadButtonType::South)))
                ),
            },
            axes: enum_map! {
                Axis::Horizontal => AxisBinding::new(
                    AnalogTrigger::DigitalJoystick{
                        negative: DigitalTrigger::Key(KeyCode::A),
                        positive: DigitalTrigger::Key(KeyCode::D),
                    },
                    None
                ),
            },

            trigger_record: TriggerRecord::new(),
        }
    }
}

impl InputBinder {
    pub fn load_from_str(&mut self, string: &str) -> Result<(), ron::Error> {
        *self = ron::from_str(string)?;
        Ok(())
    }

    pub fn axis_value(&self, axis: Axis) -> AxisState {
        let bindings: &AxisBinding = &self.axes[axis];
        self.analog_trigger_value(&bindings.primary)
            .or_else(|| {
                bindings
                    .secondary
                    .as_ref()
                    .and_then(|secondary| self.analog_trigger_value(secondary))
            })
            .unwrap_or_default()
    }

    fn analog_trigger_value(&self, trigger: &AnalogTrigger) -> Option<AxisState> {
        match trigger {
            AnalogTrigger::DigitalJoystick { negative, positive } => {
                let is_negative_pressed = self.trigger_record.state(negative).is_pressed();
                let is_positive_pressed = self.trigger_record.state(positive).is_pressed();
                match (is_negative_pressed, is_positive_pressed) {
                    (true, false) => Some(AxisState(-1.0)),
                    (false, true) => Some(AxisState(1.0)),
                    _ => None,
                }
            }
            AnalogTrigger::GamepadAxis(axis) => {
                let val = self.trigger_record.axis_values[axis];
                if val.value() == 0.0 {
                    None
                } else {
                    Some(val)
                }
            }
        }
    }

    pub fn action_value(&self, action: Action) -> ActionState {
        let bindings: &ActionBinding = &self.actions[action];

        let primary = self.trigger_record.state(&bindings.primary);

        let secondary = bindings
            .secondary
            .as_ref()
            .map(|secondary| self.trigger_record.state(secondary));

        if let Some(secondary) = secondary {
            primary.max(secondary)
        } else {
            primary
        }
    }
}

fn update_mouse_input(
    mut input_binder: ResMut<InputBinder>,
    mut events: EventReader<MouseButtonInput>,
) {
    for event in events.iter() {
        let trigger = DigitalTrigger::MouseButton(event.button);
        match event.state {
            bevy::input::ElementState::Pressed => input_binder.trigger_record.press(trigger),
            bevy::input::ElementState::Released => input_binder.trigger_record.release(trigger),
        }
    }
}

fn update_keyboard_input(
    mut input_binder: ResMut<InputBinder>,
    mut events: EventReader<KeyboardInput>,
) {
    for (state, keycode) in events
        .iter()
        .filter_map(|event| event.key_code.map(|keycode| (event.state, keycode)))
    {
        let trigger = DigitalTrigger::Key(keycode);
        match state {
            bevy::input::ElementState::Pressed => input_binder.trigger_record.press(trigger),
            bevy::input::ElementState::Released => input_binder.trigger_record.release(trigger),
        }
    }
}

fn update_gamepad_input(
    mut input_binder: ResMut<InputBinder>,
    settings: Res<GamepadSettings>,
    mut events: EventReader<GamepadEvent>,
) {
    for event in events.iter() {
        match event.1 {
            GamepadEventType::Connected => (),
            GamepadEventType::Disconnected => (),
            GamepadEventType::ButtonChanged(ty, state) => {
                let button = GamepadButton(event.0, ty);
                input_binder.trigger_record.update_gamepad_button(
                    button,
                    state,
                    settings.get_button_settings(button),
                );
            }
            GamepadEventType::AxisChanged(ty, state) => {
                let axis = GamepadAxis(event.0, ty);
                input_binder.trigger_record.update_gamepad_axis(
                    axis,
                    state,
                    settings.get_axis_settings(axis),
                );
            }
        }
    }
}

fn update_trigger_record(mut input_binder: ResMut<InputBinder>) {
    input_binder.trigger_record.finish_frame();
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct InputBindStage;
impl StageLabel for InputBindStage {
    fn dyn_clone(&self) -> std::boxed::Box<(dyn bevy::prelude::StageLabel + 'static)> {
        Box::new(*self)
    }
}

pub struct InputBindingPlugin;

impl Plugin for InputBindingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBinder>()
            .add_stage_after(
                CoreStage::PreUpdate,
                InputBindStage,
                SystemStage::parallel(),
            )
            .add_system_to_stage(InputBindStage, update_mouse_input)
            .add_system_to_stage(InputBindStage, update_keyboard_input)
            .add_system_to_stage(InputBindStage, update_gamepad_input)
            .add_system_to_stage(CoreStage::Last, update_trigger_record)
            .add_system(debug_input_bindings);
    }
}

pub fn debug_input_bindings(mut egui: ResMut<bevy_egui::EguiContext>, bindings: Res<InputBinder>) {
    let ctx = egui.ctx_mut();

    use bevy_egui::egui;
    egui::Window::new("Input bindings [debug]").show(ctx, |ui| {
        ui.label(format!("{:#?}", bindings.as_ref()));
    });
}
