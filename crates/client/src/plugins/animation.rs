use bevy::prelude::*;
use halestorm_common::types::Direction;

use super::player::{LocalPlayer, PlayerMovement};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_player_sprite);
    }
}

/// LPC spritesheet layout constants.
/// Standard LPC sheets are 832x1344 (13 columns x 21 rows of 64x64 frames).
/// Walk animations: row 8 = up, row 9 = left, row 10 = down, row 11 = right.
/// Each walk row has 9 frames. Frame 0 is the idle/standing pose.
const LPC_COLUMNS: u32 = 13;
const LPC_ROWS: u32 = 21;
const LPC_WALK_UP_ROW: u32 = 8;
const LPC_WALK_LEFT_ROW: u32 = 9;
const LPC_WALK_DOWN_ROW: u32 = 10;
const LPC_WALK_RIGHT_ROW: u32 = 11;
const LPC_WALK_FRAMES: usize = 9;

/// Component tracking the current animation state.
#[derive(Component)]
pub struct SpriteAnimation {
    /// Current frame index within the walk cycle (0-8).
    pub frame: usize,
    /// Timer controlling frame rate.
    pub timer: Timer,
    /// Last direction the character was facing (for idle frame).
    pub facing: Direction,
}

impl Default for SpriteAnimation {
    fn default() -> Self {
        Self {
            frame: 0,
            timer: Timer::from_seconds(0.08, TimerMode::Repeating),
            facing: Direction::South,
        }
    }
}

/// Returns the LPC spritesheet row for a given direction.
fn direction_to_row(direction: Direction) -> u32 {
    match direction {
        Direction::North | Direction::NorthEast | Direction::NorthWest => LPC_WALK_UP_ROW,
        Direction::South | Direction::SouthEast | Direction::SouthWest => LPC_WALK_DOWN_ROW,
        Direction::West => LPC_WALK_LEFT_ROW,
        Direction::East => LPC_WALK_RIGHT_ROW,
    }
}

/// Returns the atlas index for a given row and frame.
fn atlas_index(row: u32, frame: usize) -> usize {
    (row * LPC_COLUMNS + frame as u32) as usize
}

/// Creates the texture atlas layout for an LPC spritesheet.
pub fn lpc_atlas_layout() -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(UVec2::new(64, 64), LPC_COLUMNS, LPC_ROWS, None, None)
}

/// Returns the idle atlas index for a given direction.
pub fn idle_index(direction: Direction) -> usize {
    atlas_index(direction_to_row(direction), 0)
}

fn animate_player_sprite(
    time: Res<Time>,
    player_mov: Option<Res<PlayerMovement>>,
    mut query: Query<(&mut SpriteAnimation, &mut Sprite), With<LocalPlayer>>,
) {
    let Some(mov) = player_mov else {
        return;
    };
    let Ok((mut anim, mut sprite)) = query.single_mut() else {
        return;
    };

    if mov.is_moving() {
        anim.facing = mov.direction;
        anim.timer.tick(time.delta());

        if anim.timer.just_finished() {
            anim.frame = (anim.frame + 1) % LPC_WALK_FRAMES;
            // Skip frame 0 during walk (it's the idle pose) — cycle 1..=8
            if anim.frame == 0 {
                anim.frame = 1;
            }
        }

        let row = direction_to_row(mov.direction);
        let index = atlas_index(row, anim.frame);
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    } else {
        // Idle: show standing frame for the last direction faced
        anim.frame = 0;
        let row = direction_to_row(anim.facing);
        let index = atlas_index(row, 0);
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = index;
        }
    }
}
