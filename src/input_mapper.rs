use std::collections::HashSet;

use bevy::{
    asset::{AssetLoader, LoadedAsset},
    input::{
        gamepad::{AxisSettings, ButtonSettings, GamepadSettings},
        keyboard::KeyboardInput,
        mouse::MouseButtonInput,
    },
    prelude::*,
    reflect::TypeUuid,
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
pub enum DigitalTrigger {
    Key(KeyCode),
    MouseButton(MouseButton),
    GamepadButton(GamepadButton),
}

/// Triggers that have a state defined by a value in the `-1f32..1f32` range.
#[derive(Deserialize, Debug)]
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

impl Default for ActionState {
    fn default() -> Self {
        ActionState::Released
    }
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

#[derive(Debug, Default)]
struct TriggerRecord {
    just_pressed: HashSet<DigitalTrigger>,
    held: HashSet<DigitalTrigger>,
    just_released: HashSet<DigitalTrigger>,

    axis_values: HashMap<GamepadAxis, AxisState>,
}

impl TriggerRecord {
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

    fn digital_trigger_state(&self, trigger: &DigitalTrigger) -> ActionState {
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

    fn analog_trigger_state(&self, trigger: &AnalogTrigger) -> Option<AxisState> {
        match trigger {
            AnalogTrigger::DigitalJoystick { negative, positive } => {
                let is_negative_pressed = self.digital_trigger_state(negative).is_pressed();
                let is_positive_pressed = self.digital_trigger_state(positive).is_pressed();
                match (is_negative_pressed, is_positive_pressed) {
                    (true, false) => Some(AxisState(-1.0)),
                    (false, true) => Some(AxisState(1.0)),
                    _ => None,
                }
            }
            AnalogTrigger::GamepadAxis(axis) => {
                let val = self.axis_values.get(axis).copied().unwrap_or_default();
                if val.value() == 0.0 {
                    None
                } else {
                    Some(val)
                }
            }
        }
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

#[derive(Deserialize, TypeUuid, Debug)]
#[uuid = "a1803d98-65db-4be4-af14-eecc3af818ee"]
pub struct InputMappings {
    actions: EnumMap<Action, ActionBinding>,
    axes: EnumMap<Axis, AxisBinding>,
}

impl Default for InputMappings {
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
        }
    }
}

pub struct InputMappingsLoader;

impl AssetLoader for InputMappingsLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let loaded_asset = LoadedAsset::new(ron::de::from_bytes::<InputMappings>(bytes)?);
            load_context.set_default_asset(loaded_asset);

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

/// An intermediate resource that holds a handle to the input mappings to use as well as the trigger record.
///
/// Data from the mapper is uploaded to the [`Input`] resource, ready to use.
#[derive(Debug, Default)]
pub struct InputMapper {
    pub mappings: Handle<InputMappings>,
    trigger_record: TriggerRecord,
}

/// A resource holding the mapped input state for this frame.
#[derive(Default)]
pub struct Input {
    pub actions: EnumMap<Action, ActionState>,
    pub axes: EnumMap<Axis, AxisState>,
}

fn upload_input(
    mapper: Res<InputMapper>,
    bindings: Res<Assets<InputMappings>>,
    mut input: ResMut<Input>,
) {
    let mappings = if let Some(x) = bindings.get(&mapper.mappings) {
        x
    } else {
        return;
    };

    mappings.axes.iter().for_each(|(axis, bindings)| {
        input.axes[axis] = mapper
            .trigger_record
            .analog_trigger_state(&bindings.primary)
            .or_else(|| {
                bindings
                    .secondary
                    .as_ref()
                    .and_then(|secondary| mapper.trigger_record.analog_trigger_state(secondary))
            })
            .unwrap_or_default();
    });
    mappings.actions.iter().for_each(|(action, bindings)| {
        let primary = mapper
            .trigger_record
            .digital_trigger_state(&bindings.primary);

        let secondary = bindings
            .secondary
            .as_ref()
            .map(|secondary| mapper.trigger_record.digital_trigger_state(secondary));

        input.actions[action] = if let Some(secondary) = secondary {
            primary.max(secondary)
        } else {
            primary
        }
    });
}

fn update_mouse_input(
    mut input_binder: ResMut<InputMapper>,
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
    mut input_binder: ResMut<InputMapper>,
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
    mut input_binder: ResMut<InputMapper>,
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

fn update_trigger_record(mut input_binder: ResMut<InputMapper>) {
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
        app.init_resource::<InputMapper>()
            .init_resource::<Input>()
            .add_asset::<InputMappings>()
            .add_asset_loader(InputMappingsLoader)
            .add_stage_after(
                CoreStage::PreUpdate,
                InputBindStage,
                SystemStage::parallel(),
            )
            .add_system_set_to_stage(
                InputBindStage,
                SystemSet::new()
                    .with_system(update_mouse_input)
                    .with_system(update_keyboard_input)
                    .with_system(update_gamepad_input)
                    .label("update input"),
            )
            .add_system_to_stage(InputBindStage, upload_input.after("update input"))
            .add_system_to_stage(CoreStage::Last, update_trigger_record)
            .add_system(debug_input_bindings);
    }
}

pub fn debug_input_bindings(mut egui: ResMut<bevy_egui::EguiContext>, bindings: Res<InputMapper>) {
    let ctx = egui.ctx_mut();

    use bevy_egui::egui;
    egui::Window::new("Input bindings [debug]").show(ctx, |ui| {
        ui.label(format!("{:#?}", bindings.as_ref()));
    });
}
