use std::collections::HashSet;

use enum_map::{enum_map, Enum, EnumMap};
use ggez::*;

/// Triggers that have a state defined by an [ActionState] value.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum DigitalTrigger {
    Key {
        key: input::keyboard::KeyCode,
        mods: input::keyboard::KeyMods,
    },
    MouseButton(input::mouse::MouseButton),
    GamepadButton {
        id: input::gamepad::gilrs::Button,
    },
}

/// Triggers that have a state defined by a value in the `-1f32..1f32` range.
pub enum AnalogTrigger {
    /// Emulates a real joystick axis with two keys.
    ///
    /// - If `negative` is pressed, then its state will be -1.
    /// - If `positive` is pressed, then its state will be 1.
    /// - If both or none are pressed, then its state will be 0.
    KeyJoystick {
        negative: input::keyboard::KeyCode,
        positive: input::keyboard::KeyCode,
    },
    GamepadAxis {
        gamepad_id: input::gamepad::GamepadId,
        axis: input::gamepad::gilrs::Axis,
        deadzone: f32,
    },
}

impl AnalogTrigger {
    fn value(&self, ctx: &Context) -> AxisState {
        AxisState(match self {
            AnalogTrigger::KeyJoystick { negative, positive } => {
                match (
                    input::keyboard::is_key_pressed(ctx, *negative),
                    input::keyboard::is_key_pressed(ctx, *positive),
                ) {
                    (true, true) | (false, false) => 0.,
                    (true, false) => -1.,
                    (false, true) => 1.,
                }
            }
            AnalogTrigger::GamepadAxis {
                gamepad_id,
                axis,
                deadzone,
            } => {
                let val = input::gamepad::gamepad(ctx, *gamepad_id)
                    .axis_data(*axis)
                    .unwrap()
                    .value()
                    / 100.;
                if &val.abs() < deadzone {
                    0.
                } else {
                    val
                }
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
#[non_exhaustive]
pub enum Action {
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionState {
    Released,
    JustReleased,
    JustPressed,
    Held,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
#[non_exhaustive]
pub enum Axis {
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AxisState(f32);

impl AxisState {
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl ActionState {
    pub fn is_pressed(self) -> bool {
        matches!(self, Self::JustPressed | Self::Held)
    }
}

struct DigitalTriggerRecord {
    just_pressed: HashSet<DigitalTrigger>,
    held: HashSet<DigitalTrigger>,
    just_released: HashSet<DigitalTrigger>,
}

impl DigitalTriggerRecord {
    fn new() -> Self {
        Self {
            just_pressed: HashSet::new(),
            held: HashSet::new(),
            just_released: HashSet::new(),
        }
    }

    fn press(&mut self, trigger: DigitalTrigger) {
        self.just_pressed.insert(trigger);
    }

    fn release(&mut self, trigger: DigitalTrigger) {
        self.held.retain(|x| match (x, &trigger) {
            (DigitalTrigger::Key { key: l_key, .. }, DigitalTrigger::Key { key: r_key, .. }) => {
                l_key != r_key
            }
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

/// A binding from an input source to an action.
pub struct ActionBinding {
    main: DigitalTrigger,
    secondary: Option<DigitalTrigger>,
}

impl ActionBinding {
    pub fn new(main: DigitalTrigger, secondary: Option<DigitalTrigger>) -> Self {
        Self { main, secondary }
    }

    fn state(&self, record: &DigitalTriggerRecord) -> ActionState {
        let main_state = record.state(&self.main);
        let secondary_state = self.secondary.as_ref().map(|trigger| record.state(trigger));
        match (main_state, secondary_state) {
            (main_state, None) => main_state,
            (main_state, Some(secondary_state)) => ActionState::max(main_state, secondary_state),
        }
    }
}

/// A binding from an input source to an axis.
pub struct AxisBinding {
    main: AnalogTrigger,
    secondary: Option<AnalogTrigger>,
}

impl AxisBinding {
    pub fn new(main: AnalogTrigger, secondary: Option<AnalogTrigger>) -> Self {
        Self { main, secondary }
    }

    fn state(&self, ctx: &Context) -> AxisState {
        let main_state = self.main.value(ctx);
        let secondary_state = self.secondary.as_ref().map(|trigger| trigger.value(ctx));
        match (main_state, secondary_state) {
            (main_state, None) => main_state,
            (main_state, Some(secondary_state)) if main_state.value() == 0. => secondary_state,
            (main_state, Some(_)) => main_state,
        }
    }
}

pub struct InputBinder {
    actions: EnumMap<Action, ActionBinding>,
    axes: EnumMap<Axis, AxisBinding>,

    trigger_record: DigitalTriggerRecord,
    enabled: bool,
}

impl Default for InputBinder {
    fn default() -> Self {
        Self {
            actions: enum_map! {
                Action::Jump => ActionBinding::new(
                        DigitalTrigger::Key{
                            key: input::keyboard::KeyCode::Space,
                            mods: input::keyboard::KeyMods::default()
                        }, Some(DigitalTrigger::GamepadButton{id: ggez::event::Button::South})
                ),
            },
            axes: enum_map! {
                Axis::Horizontal => AxisBinding::new(
                    AnalogTrigger::KeyJoystick{
                        negative: input::keyboard::KeyCode::A,
                        positive: input::keyboard::KeyCode::D
                    },
                    None
                ),
            },

            trigger_record: DigitalTriggerRecord::new(),

            enabled: true,
        }
    }
}

impl InputBinder {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn key_down_event(
        &mut self,
        keycode: event::KeyCode,
        keymods: event::KeyMods,
        repeat: bool,
    ) {
        if !repeat {
            self.trigger_record.press(DigitalTrigger::Key {
                key: keycode,
                mods: keymods,
            });
        }
    }

    pub fn key_up_event(&mut self, keycode: event::KeyCode, keymods: event::KeyMods) {
        self.trigger_record.release(DigitalTrigger::Key {
            key: keycode,
            mods: keymods,
        });
    }

    pub fn mouse_button_down_event(&mut self, button: input::mouse::MouseButton) {
        self.trigger_record
            .press(DigitalTrigger::MouseButton(button));
    }

    pub fn mouse_button_up_event(&mut self, button: input::mouse::MouseButton) {
        self.trigger_record
            .release(DigitalTrigger::MouseButton(button));
    }

    pub fn gamepad_button_down_event(&mut self, btn: event::Button, _id: event::GamepadId) {
        self.trigger_record
            .press(DigitalTrigger::GamepadButton { id: btn });
    }

    pub fn gamepad_button_up_event(&mut self, btn: event::Button, _id: event::GamepadId) {
        self.trigger_record
            .release(DigitalTrigger::GamepadButton { id: btn });
    }

    /// Convert "just pressed" triggers to held ones and clear "just released" triggers
    pub fn finish_frame(&mut self) {
        self.trigger_record.finish_frame();
    }

    pub fn action(&self, action: Action) -> ActionState {
        // HACK: This will not send [ActionState::JustReleased] if the input binder is disabled while holding a trigger
        if self.enabled {
            self.actions[action].state(&self.trigger_record)
        } else {
            ActionState::Released
        }
    }

    pub fn axis(&self, ctx: &Context, axis: Axis) -> AxisState {
        if self.enabled {
            self.axes[axis].state(ctx)
        } else {
            AxisState(0.)
        }
    }
}