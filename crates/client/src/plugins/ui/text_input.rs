use bevy::{input::keyboard::{Key, KeyboardInput}, prelude::*};

/// Marker for a text input field. Attach to a Text entity.
#[derive(Component)]
pub struct TextInput {
    pub value: String,
    pub placeholder: String,
    pub focused: bool,
    pub password: bool,
}

impl TextInput {
    pub fn new(placeholder: &str) -> Self {
        Self {
            value: String::new(),
            placeholder: placeholder.into(),
            focused: false,
            password: false,
        }
    }

    pub fn password(placeholder: &str) -> Self {
        Self {
            password: true,
            ..Self::new(placeholder)
        }
    }

    pub fn display_text(&self) -> String {
        if self.value.is_empty() {
            return self.placeholder.clone();
        }
        if self.password {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        }
    }
}

/// System that processes keyboard input for focused TextInput fields.
pub fn update_text_inputs(
    mut keyboard_reader: MessageReader<KeyboardInput>,
    mut query: Query<(&mut TextInput, &mut Text)>,
) {
    for event in keyboard_reader.read() {
        if !event.state.is_pressed() {
            continue;
        }

        for (mut input, mut text) in &mut query {
            if !input.focused {
                continue;
            }

            match (&event.logical_key, &event.text) {
                (Key::Backspace, _) => {
                    input.value.pop();
                }
                (Key::Tab | Key::Enter, _) => {
                    // Handled by parent UI
                }
                (_, Some(inserted)) => {
                    if inserted.chars().all(|c| !c.is_ascii_control()) {
                        input.value.push_str(inserted);
                    }
                }
                _ => {}
            }

            let display = input.display_text();
            let cursor = if input.focused { "|" } else { "" };
            **text = format!("{display}{cursor}");
        }
    }
}

/// System to handle clicking on text inputs to focus them.
pub fn handle_text_input_focus(
    interactions: Query<(&Interaction, &TextInputLink), Changed<Interaction>>,
    mut inputs: Query<&mut TextInput>,
) {
    for (interaction, link) in &interactions {
        if *interaction == Interaction::Pressed {
            // Unfocus all, then focus the clicked one
            for mut input in &mut inputs {
                input.focused = false;
            }
            if let Ok(mut input) = inputs.get_mut(link.0) {
                input.focused = true;
            }
        }
    }
}

/// Links a clickable UI node to its TextInput entity.
#[derive(Component)]
pub struct TextInputLink(pub Entity);

/// Handle Tab key to cycle focus between text inputs.
pub fn handle_tab_focus(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut inputs: Query<(Entity, &mut TextInput)>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }

    let entities: Vec<Entity> = inputs.iter().map(|(e, _)| e).collect();
    if entities.is_empty() {
        return;
    }

    let current_focused = inputs.iter().find(|(_, i)| i.focused).map(|(e, _)| e);

    // Unfocus all
    for (_, mut input) in &mut inputs {
        input.focused = false;
    }

    // Focus next (or first if none focused)
    let next = match current_focused {
        Some(current) => {
            let idx = entities.iter().position(|&e| e == current).unwrap_or(0);
            entities[(idx + 1) % entities.len()]
        }
        None => entities[0],
    };

    if let Ok((_, mut input)) = inputs.get_mut(next) {
        input.focused = true;
    }
}
