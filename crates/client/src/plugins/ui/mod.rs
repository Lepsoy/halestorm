pub mod login;
pub mod character_create;
pub mod hud;
mod text_input;

use bevy::prelude::*;

use self::character_create::CharacterCreatePlugin;
use self::hud::HudPlugin;
use self::login::LoginPlugin;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameScreen>()
            .add_plugins(LoginPlugin)
            .add_plugins(CharacterCreatePlugin)
            .add_plugins(HudPlugin);
    }
}

/// Game screen states controlling which UI is shown.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameScreen {
    #[default]
    Login,
    CharacterCreate,
    InGame,
}
