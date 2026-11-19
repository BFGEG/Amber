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

    pub fn fixed_update(&mut self, movement: Vec3, dt: f32) {
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

/// Ограничивает работу симуляции после задержек, сохраняя остаток обычного кадра.
#[derive(Default)]
pub struct FixedClock {
    accumulated: f64,
}

impl FixedClock {
    pub const STEP: f64 = 1.0 / 60.0;
    const MAX_FRAME: f64 = 0.25;
    const MAX_STEPS: usize = 8;

    pub fn advance(&mut self, frame_dt: f32) -> usize {
        let frame_dt = f64::from(frame_dt);
        if !frame_dt.is_finite() || !(0.0..=Self::MAX_FRAME).contains(&frame_dt) {
            // Не догоняем время сворачивания окна или длительной остановки.
            self.accumulated = 0.0;
            return 0;
        }
        self.accumulated += frame_dt;
        let available = (self.accumulated / Self::STEP).floor() as usize;
        let steps = available.min(Self::MAX_STEPS);
        // Избыточные целые шаги отбрасываем: долг не переходит в следующий кадр.
        self.accumulated -= available as f64 * Self::STEP;
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn travel(fps: u32) -> Player {
        let mut player = Player::new(Vec3::ZERO);
        let mut clock = FixedClock::default();
        for _ in 0..fps * 3 {
            for _ in 0..clock.advance(1.0 / fps as f32) {
                player.fixed_update(Vec3::X, FixedClock::STEP as f32);
            }
        }
        player
    }

    #[test]
    fn movement_matches_at_different_render_rates() {
        for fps in [30, 60, 144] {
            assert!((travel(fps).position.x - MOVE_SPEED * 3.0).abs() < 0.001);
        }
    }

    #[test]
    fn diagonal_does_not_increase_speed() {
        let mut straight = Player::new(Vec3::ZERO);
        let mut diagonal = Player::new(Vec3::ZERO);
        straight.fixed_update(Vec3::X, FixedClock::STEP as f32);
        diagonal.fixed_update(Vec3::X + Vec3::Z, FixedClock::STEP as f32);
        assert!((straight.position.length() - diagonal.position.length()).abs() < 0.00001);
    }

    #[test]
    fn stalled_frame_does_not_move_player_or_leave_time_debt() {
        let mut clock = FixedClock::default();
        clock.advance(0.01);
        assert_eq!(clock.advance(5.0), 0);
        assert_eq!(clock.advance(1.0 / 60.0), 1);
    }

    #[test]
    fn slow_frame_has_bounded_work_without_catch_up_next_frame() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(0.25), FixedClock::MAX_STEPS);
        assert_eq!(clock.advance(1.0 / 60.0), 1);
    }

    #[test]
    fn invalid_time_does_not_poison_following_frames() {
        let mut clock = FixedClock::default();
        for dt in [f32::NAN, f32::INFINITY, -1.0] {
            assert_eq!(clock.advance(dt), 0);
            assert_eq!(clock.advance(1.0 / 60.0), 1);
        }
    }
}
