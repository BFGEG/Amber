use std::f32::consts::PI;

use bfge::math::{Quat, Vec3};

const TURN_SMOOTHING: f32 = 10.0;
const MOVE_SPEED: f32 = 13.0;

/// Игровое положение персонажа независимо от узлов графической сцены.
pub struct Player {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
        }
    }

    pub fn update(&mut self, movement: Vec3, dt: f32) {
        if movement.length_squared() == 0.0 {
            return;
        }
        let movement = movement.normalize();
        let target_rotation = Quat::from_rotation_y(movement.x.atan2(movement.z) + PI / 2.0);
        let turn_blend = 1.0 - (-TURN_SMOOTHING * dt).exp();
        self.rotation = self.rotation.slerp(target_rotation, turn_blend);
        self.position += movement * MOVE_SPEED * dt;
    }
}
