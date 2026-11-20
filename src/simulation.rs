use std::f32::consts::PI;

use bfge::math::{Quat, Vec3};

use crate::collision;
use crate::world::Aabb;

const TURN_SMOOTHING: f32 = 10.0;
const MOVE_SPEED: f32 = 13.0;
const GRAVITY: f32 = 24.0;

/// Игровое положение персонажа независимо от узлов графической сцены.
pub struct Player {
    pub position: Vec3,
    pub rotation: Quat,
    pub vertical_velocity: f32,
    pub grounded: bool,
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            vertical_velocity: 0.0,
            grounded: false,
        }
    }

    pub fn fixed_update(&mut self, movement: Vec3, dt: f32, obstacles: &[Aabb]) {
        self.grounded =
            self.vertical_velocity <= 0.0 && collision::has_support(self.position, obstacles);
        if self.grounded {
            self.vertical_velocity = 0.0;
        }
        if movement.length_squared() > 0.0 {
            let movement = movement.normalize();
            let target_rotation = Quat::from_rotation_y(movement.x.atan2(movement.z) + PI / 2.0);
            let turn_blend = 1.0 - (-TURN_SMOOTHING * dt).exp();
            self.rotation = self.rotation.slerp(target_rotation, turn_blend);
            self.position =
                collision::move_horizontal(self.position, movement * MOVE_SPEED * dt, obstacles);
        }
        let fall = self.vertical_velocity * dt - 0.5 * GRAVITY * dt * dt;
        self.vertical_velocity -= GRAVITY * dt;
        let (position, contact) = collision::move_vertical(self.position, fall, obstacles);
        self.position = position;
        self.grounded = contact == Some(collision::VerticalContact::Ground);
        if contact.is_some() {
            self.vertical_velocity = 0.0;
        }
    }
}

/// Ограничивает работу симуляции после задержек, сохраняя остаток обычного кадра.
#[derive(Default)]
pub struct FixedClock {
    accumulated: f64,
    was_focused: bool,
}

impl FixedClock {
    pub const STEP: f64 = 1.0 / 60.0;
    const MAX_FRAME: f64 = 0.25;
    const MAX_STEPS: usize = 8;

    /// Возвращает число шагов либо пропуск всей игры и камеры при паузе.
    pub fn frame_steps(&mut self, frame_dt: f32, focused: bool) -> Option<usize> {
        let resuming = !self.was_focused;
        self.was_focused = focused;
        if !focused || resuming {
            self.accumulated = 0.0;
            None
        } else {
            Some(self.advance(frame_dt))
        }
    }

    fn advance(&mut self, frame_dt: f32) -> usize {
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

    #[test]
    fn stationary_player_lands_and_remains_on_the_ground() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::new(0.0, 5.0, 0.0));
        for _ in 0..600 {
            player.fixed_update(Vec3::ZERO, FixedClock::STEP as f32, &obstacles);
        }
        assert_eq!(player.position.y, 0.0);
        assert_eq!(player.vertical_velocity, 0.0);
        assert!(player.grounded);
    }

    #[test]
    fn leaving_platform_loses_support_and_lands_on_lower_ground() {
        let platform = Aabb {
            min: Vec3::new(-1.0, 0.0, -1.0),
            max: Vec3::new(1.0, 2.0, 1.0),
        };
        let obstacles = [crate::world::GROUND.bounds(), platform];
        let mut player = Player::new(Vec3::new(0.0, 2.0, 0.0));
        for _ in 0..10 {
            player.fixed_update(Vec3::X, FixedClock::STEP as f32, &obstacles);
        }
        assert!(!player.grounded);
        assert!(player.position.y < 2.0);
        for _ in 0..120 {
            player.fixed_update(Vec3::ZERO, FixedClock::STEP as f32, &obstacles);
        }
        assert_eq!(player.position.y, 0.0);
        assert!(player.grounded);
    }

    #[test]
    fn pressing_against_wall_does_not_raise_player() {
        let obstacles = [
            crate::world::GROUND.bounds(),
            Aabb {
                min: Vec3::new(2.0, 0.0, -10.0),
                max: Vec3::new(3.0, 4.0, 10.0),
            },
        ];
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..120 {
            player.fixed_update(Vec3::X, FixedClock::STEP as f32, &obstacles);
        }
        assert_eq!(player.position.y, 0.0);
        assert!(player.grounded);
        assert!(player.position.x < 1.5);
    }

    fn travel(fps: u32) -> Player {
        let mut player = Player::new(Vec3::ZERO);
        let mut clock = FixedClock::default();
        for _ in 0..fps * 3 {
            for _ in 0..clock.advance(1.0 / fps as f32) {
                player.fixed_update(Vec3::X, FixedClock::STEP as f32, &[]);
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
    fn focus_pause_freezes_player_and_discards_partial_step() {
        let mut clock = FixedClock::default();
        let mut player = Player::new(Vec3::ZERO);
        assert_eq!(clock.frame_steps(0.02, true), None);
        assert_eq!(clock.frame_steps(0.01, true), Some(0));
        for (dt, focused) in [(0.02, false), (10.0, false), (0.2, true)] {
            for _ in 0..clock.frame_steps(dt, focused).unwrap_or(0) {
                player.fixed_update(Vec3::X, FixedClock::STEP as f32, &[]);
            }
            assert_eq!(player.position, Vec3::ZERO);
            assert_eq!(player.rotation, Quat::IDENTITY);
        }
        assert_eq!(clock.frame_steps(0.01, true), Some(0));
        assert_eq!(clock.frame_steps(0.01, true), Some(1));
    }

    #[test]
    fn wall_contact_matches_at_different_frame_rates() {
        let obstacles = [
            crate::world::GROUND.bounds(),
            Aabb {
                min: Vec3::new(2.0, 0.0, -100.0),
                max: Vec3::new(2.1, 2.0, 100.0),
            },
        ];
        for fps in [30, 60, 144] {
            let mut player = Player::new(Vec3::ZERO);
            let mut clock = FixedClock::default();
            for _ in 0..fps * 3 {
                for _ in 0..clock.advance(1.0 / fps as f32) {
                    player.fixed_update(Vec3::X + Vec3::Z, FixedClock::STEP as f32, &obstacles);
                }
            }
            assert!((player.position.x - 1.5).abs() < 0.001);
            assert!((player.position.z - MOVE_SPEED * 3.0 / 2.0_f32.sqrt()).abs() < 0.001);
            player.fixed_update(-Vec3::X, FixedClock::STEP as f32, &obstacles);
            assert!(player.position.x < 1.4);
        }
    }

    #[test]
    fn diagonal_does_not_increase_speed() {
        let mut straight = Player::new(Vec3::ZERO);
        let mut diagonal = Player::new(Vec3::ZERO);
        straight.fixed_update(Vec3::X, FixedClock::STEP as f32, &[]);
        diagonal.fixed_update(Vec3::X + Vec3::Z, FixedClock::STEP as f32, &[]);
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
