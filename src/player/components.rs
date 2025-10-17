use bevy::prelude::*;

pub(crate) const TILE_SIZE: u32 = 64;
pub(crate) const WALK_FRAMES: usize = 9;
pub(crate) const MOVE_SPEED: f32 = 280.0;
pub(crate) const ANIM_DT: f32 = 0.1;
pub(crate) const PLAYER_Z: f32 = 20.0;

#[derive(Component)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Facing {
    Up,
    Left,
    Down,
    Right,
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct AnimationTimer(pub Timer);

#[derive(Component)]
pub(crate) struct AnimationState {
    pub(crate) facing: Facing,
    pub(crate) moving: bool,
    pub(crate) was_moving: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct AnimationClip {
    first: usize,
    last: usize,
}

impl AnimationClip {
    pub(crate) fn from_row(row: usize, frames_per_row: usize) -> Self {
        let first = row * frames_per_row;
        Self {
            first,
            last: first + frames_per_row - 1,
        }
    }

    pub(crate) fn start(self) -> usize {
        self.first
    }

    pub(crate) fn contains(self, index: usize) -> bool {
        (self.first..=self.last).contains(&index)
    }

    pub(crate) fn next(self, index: usize) -> usize {
        if index >= self.last {
            self.first
        } else {
            index + 1
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) struct DirectionalClips {
    up: AnimationClip,
    left: AnimationClip,
    down: AnimationClip,
    right: AnimationClip,
}

impl DirectionalClips {
    pub(crate) fn walk(frames_per_row: usize) -> Self {
        Self {
            up: AnimationClip::from_row(row_zero_based(Facing::Up), frames_per_row),
            left: AnimationClip::from_row(row_zero_based(Facing::Left), frames_per_row),
            down: AnimationClip::from_row(row_zero_based(Facing::Down), frames_per_row),
            right: AnimationClip::from_row(row_zero_based(Facing::Right), frames_per_row),
        }
    }

    pub(crate) fn clip(&self, facing: Facing) -> AnimationClip {
        match facing {
            Facing::Up => self.up,
            Facing::Left => self.left,
            Facing::Down => self.down,
            Facing::Right => self.right,
        }
    }
}

fn row_zero_based(facing: Facing) -> usize {
    match facing {
        Facing::Up => 8,
        Facing::Left => 9,
        Facing::Down => 10,
        Facing::Right => 11,
    }
}
