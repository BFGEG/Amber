use std::f32::consts::PI;

use bfge::math::{Quat, Vec3};

use crate::collision;
use crate::world::Aabb;

const TURN_SMOOTHING: f32 = 10.0;
const MOVE_SPEED: f32 = 7.0;
const PRECISE_SPEED: f32 = 3.0;
const GROUND_ACCELERATION: f32 = 40.0;
const GROUND_BRAKING: f32 = 50.0;
const AIR_ACCELERATION: f32 = 40.0;
const GRAVITY: f32 = 24.0;
const JUMP_SPEED: f32 = 8.0;
pub const FALL_LIMIT: f32 = -10.0;

/// Режим ограничения горизонтальной скорости.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeedMode {
    #[default]
    Normal,
    Precise,
}

impl SpeedMode {
    fn max_speed(self) -> f32 {
        match self {
            Self::Normal => MOVE_SPEED,
            Self::Precise => PRECISE_SPEED,
        }
    }
}

/// Игровое положение персонажа независимо от узлов графической сцены.
pub struct Player {
    pub position: Vec3,
    pub rotation: Quat,
    pub horizontal_velocity: Vec3,
    pub vertical_velocity: f32,
    pub grounded: bool,
    // После приземления на край стопа может оставаться на уступе частично.
    landed_on_edge: bool,
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            horizontal_velocity: Vec3::ZERO,
            vertical_velocity: 0.0,
            grounded: false,
            landed_on_edge: false,
        }
    }

    pub fn is_below_fall_limit(&self) -> bool {
        self.position.y < FALL_LIMIT
    }

    /// Возвращает в заранее проверенную безопасную точку и сбрасывает движение.
    pub fn respawn(&mut self, start: Vec3, obstacles: &[Aabb]) {
        *self = Self::new(start);
        self.grounded = collision::has_support(start, obstacles);
    }

    fn update_horizontal_velocity(
        &mut self,
        input_direction: Vec3,
        speed_mode: SpeedMode,
        dt: f32,
    ) {
        let has_input = input_direction.length_squared() > 0.0;
        let rate = if self.grounded {
            if has_input {
                GROUND_ACCELERATION
            } else {
                GROUND_BRAKING
            }
        } else if has_input {
            AIR_ACCELERATION
        } else {
            // В воздухе без ввода скорость сохраняется: дуга прыжка остаётся предсказуемой.
            0.0
        };
        let target_velocity = input_direction * speed_mode.max_speed();
        let velocity_delta = target_velocity - self.horizontal_velocity;
        let max_delta = rate * dt;
        if velocity_delta.length() > max_delta {
            self.horizontal_velocity += velocity_delta.normalize() * max_delta;
        } else {
            self.horizontal_velocity = target_velocity;
        }
    }

    pub fn fixed_update(
        &mut self,
        movement: Vec3,
        jump_pressed: bool,
        speed_mode: SpeedMode,
        dt: f32,
        obstacles: &[Aabb],
    ) {
        let full_support = collision::has_support(self.position, obstacles);
        let partial_support = collision::has_partial_support(self.position, obstacles);
        self.grounded = self.vertical_velocity <= 0.0
            && (full_support || (self.landed_on_edge && partial_support));
        if full_support || !self.grounded {
            self.landed_on_edge = false;
        }
        if self.grounded {
            self.vertical_velocity = 0.0;
            if jump_pressed {
                self.vertical_velocity = JUMP_SPEED;
                self.grounded = false;
                self.landed_on_edge = false;
            }
        }
        let input_direction = if movement.length_squared() > 0.0 {
            movement.normalize()
        } else {
            Vec3::ZERO
        };
        if input_direction.length_squared() > 0.0 {
            let target_rotation =
                Quat::from_rotation_y(input_direction.x.atan2(input_direction.z) + PI / 2.0);
            let turn_blend = 1.0 - (-TURN_SMOOTHING * dt).exp();
            self.rotation = self.rotation.slerp(target_rotation, turn_blend);
        }
        self.update_horizontal_velocity(input_direction, speed_mode, dt);
        let previous_position = self.position;
        self.position =
            collision::move_horizontal(self.position, self.horizontal_velocity * dt, obstacles);
        // Упор в стену не должен сохранять скорость в заблокированную ось.
        self.horizontal_velocity = (self.position - previous_position) / dt;
        let fall = self.vertical_velocity * dt - 0.5 * GRAVITY * dt * dt;
        self.vertical_velocity -= GRAVITY * dt;
        let (position, contact) =
            collision::move_vertical(self.position, fall, obstacles, self.landed_on_edge);
        self.position = position;
        self.grounded = contact == Some(collision::VerticalContact::Ground);
        if self.grounded {
            let full_support = collision::has_support(self.position, obstacles);
            self.landed_on_edge =
                !full_support && collision::has_partial_support(self.position, obstacles);
        } else {
            self.landed_on_edge = false;
        }
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

    pub fn clear_time(&mut self) {
        self.accumulated = 0.0;
    }

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

    fn respawn_if_fallen(player: &mut Player, start: Vec3, obstacles: &[Aabb]) -> bool {
        if !player.is_below_fall_limit() {
            return false;
        }
        let recovery = crate::recovery::Recovery::new(crate::recovery::CheckpointPose {
            position: start,
            yaw: 0.0,
            pitch: 0.0,
        });
        recovery.restore(
            &mut crate::section::SectionState::new(None),
            player,
            obstacles,
        );
        true
    }

    fn step(player: &mut Player, movement: Vec3, jump: bool, obstacles: &[Aabb]) {
        step_with_mode(player, movement, jump, SpeedMode::Normal, obstacles);
    }

    fn step_with_mode(
        player: &mut Player,
        movement: Vec3,
        jump: bool,
        speed_mode: SpeedMode,
        obstacles: &[Aabb],
    ) {
        player.fixed_update(
            movement,
            jump,
            speed_mode,
            FixedClock::STEP as f32,
            obstacles,
        );
    }

    fn run_steps(
        player: &mut Player,
        movement: Vec3,
        count: usize,
        jump: bool,
        obstacles: &[Aabb],
    ) {
        for step_index in 0..count {
            step(player, movement, jump && step_index == 0, obstacles);
        }
    }

    fn run_until(
        player: &mut Player,
        movement: Vec3,
        obstacles: &[Aabb],
        max_steps: usize,
        reached: impl Fn(&Player) -> bool,
    ) -> bool {
        for _ in 0..max_steps {
            step(player, movement, false, obstacles);
            if reached(player) {
                return true;
            }
        }
        false
    }

    fn jump_until_landed(
        player: &mut Player,
        direction: Vec3,
        obstacles: &[Aabb],
        max_steps: usize,
    ) -> bool {
        for step_index in 0..max_steps {
            step(player, direction, step_index == 0, obstacles);
            if step_index > 0 && player.grounded {
                return true;
            }
        }
        false
    }

    fn jump_to_platform(
        player: &mut Player,
        direction: Vec3,
        platform: crate::world::StaticBox,
        obstacles: &[Aabb],
    ) {
        assert!(jump_until_landed(player, direction, obstacles, 120));
        let bounds = platform.bounds();
        assert!(player.grounded);
        assert!(player.position.x > bounds.min.x && player.position.x < bounds.max.x);
        assert!(player.position.z > bounds.min.z && player.position.z < bounds.max.z);
        assert!((player.position.y - bounds.max.y).abs() < 0.00001);
    }

    fn travel_distance_for(speed: f32, seconds: f32) -> f32 {
        let acceleration_time = speed / GROUND_ACCELERATION;
        if seconds <= acceleration_time {
            0.5 * GROUND_ACCELERATION * seconds * seconds
        } else {
            0.5 * speed * acceleration_time + speed * (seconds - acceleration_time)
        }
    }

    fn travel_distance(seconds: f32) -> f32 {
        travel_distance_for(MOVE_SPEED, seconds)
    }

    #[test]
    fn training_course_climbs_through_the_side_turn_after_respawn() {
        let start = crate::world::START_POSITION;
        let platforms = crate::world::ASCENT;
        let lower = crate::world::LOWER_TERRACE.bounds();
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = crate::world::course_colliders();
        let mut player = Player::new(start);

        assert!(run_until(
            &mut player,
            -Vec3::X,
            &obstacles,
            120,
            |player| player.position.x <= lower.min.x - 0.1
        ));
        assert!(!player.grounded);
        assert!(player.position.y < start.y);
        assert!(player.position.x < lower.min.x);
        assert!(player.position.x > platforms[0].bounds().max.x);

        let mut returned = false;
        for _ in 0..200 {
            step(&mut player, -Vec3::X, false, &obstacles);
            if respawn_if_fallen(&mut player, start, &obstacles) {
                returned = true;
                break;
            }
        }
        assert!(returned);
        assert_eq!(player.position, start);

        assert!(run_until(
            &mut player,
            -Vec3::X,
            &obstacles,
            120,
            |player| player.position.x <= lower.min.x + foot + 0.1
        ));
        jump_to_platform(&mut player, -Vec3::X, platforms[0], &obstacles);

        assert!(run_until(
            &mut player,
            -Vec3::X,
            &obstacles,
            120,
            |player| player.position.x <= platforms[0].bounds().min.x + foot + 0.1
        ));
        jump_to_platform(&mut player, -Vec3::X, platforms[1], &obstacles);

        assert!(run_until(
            &mut player,
            -Vec3::Z,
            &obstacles,
            120,
            |player| player.position.z <= platforms[1].bounds().min.z + foot + 0.1
        ));
        jump_to_platform(&mut player, -Vec3::Z, platforms[2], &obstacles);

        assert!(run_until(
            &mut player,
            -Vec3::X,
            &obstacles,
            120,
            |player| player.position.x <= platforms[2].bounds().min.x + foot + 0.1
        ));
        jump_to_platform(&mut player, -Vec3::X, platforms[3], &obstacles);

        assert!(run_until(
            &mut player,
            -Vec3::X,
            &obstacles,
            120,
            |player| player.position.x <= platforms[3].bounds().min.x + foot + 0.1
        ));
        jump_to_platform(
            &mut player,
            -Vec3::X,
            crate::world::UPPER_TERRACE,
            &obstacles,
        );
    }

    #[test]
    fn respawn_resets_fall_and_remains_stable_at_each_scene_start() {
        let obstacles = crate::world::diagnostic_colliders();
        for start in [
            crate::world::START_POSITION,
            crate::world::TIMING_START_POSITION,
        ] {
            let mut player = Player::new(Vec3::new(200.0, FALL_LIMIT - 1.0, 0.0));
            player.horizontal_velocity = Vec3::X * MOVE_SPEED;
            player.vertical_velocity = -100.0;
            player.rotation = Quat::from_rotation_y(1.0);
            assert!(respawn_if_fallen(&mut player, start, &obstacles));
            assert_eq!(player.position, start);
            assert_eq!(player.rotation, Quat::IDENTITY);
            assert_eq!(player.horizontal_velocity, Vec3::ZERO);
            assert_eq!(player.vertical_velocity, 0.0);
            assert!(player.grounded);
            for _ in 0..600 {
                player.fixed_update(
                    Vec3::ZERO,
                    false,
                    SpeedMode::Normal,
                    FixedClock::STEP as f32,
                    &obstacles,
                );
                assert!(!respawn_if_fallen(&mut player, start, &obstacles));
                assert_eq!(player.position, start);
            }
        }
    }

    #[test]
    fn respawn_after_hazard_contact_resets_to_start() {
        let obstacles = crate::world::course_colliders();
        let mut player = Player::new(Vec3::new(-5.0, 0.0, 2.0));
        player.horizontal_velocity = Vec3::X * MOVE_SPEED;
        player.vertical_velocity = -8.0;
        player.rotation = Quat::from_rotation_y(0.5);
        player.grounded = true;

        player.respawn(crate::world::START_POSITION, &obstacles);

        assert_eq!(player.position, crate::world::START_POSITION);
        assert_eq!(player.rotation, Quat::IDENTITY);
        assert_eq!(player.horizontal_velocity, Vec3::ZERO);
        assert_eq!(player.vertical_velocity, 0.0);
        assert!(player.grounded);
    }

    #[test]
    fn return_waits_until_feet_are_below_fall_limit() {
        let mut player = Player::new(Vec3::Y * FALL_LIMIT);
        assert!(!respawn_if_fallen(
            &mut player,
            crate::world::START_POSITION,
            &[]
        ));
        player.position.y -= 0.01;
        assert!(respawn_if_fallen(
            &mut player,
            crate::world::START_POSITION,
            &crate::world::diagnostic_colliders()
        ));
    }

    #[test]
    fn each_scene_start_has_support_and_does_not_intersect_obstacles() {
        let obstacles = crate::world::diagnostic_colliders();
        for start in [
            crate::world::START_POSITION,
            crate::world::TIMING_START_POSITION,
        ] {
            assert!(collision::has_support(start, &obstacles));
            let bounds = collision::player_bounds(start);
            for obstacle in &obstacles {
                let overlap = bounds.min.cmplt(obstacle.max) & bounds.max.cmpgt(obstacle.min);
                assert!(!overlap.all());
            }
        }
    }

    #[test]
    fn repeated_falls_allow_a_new_jump_without_replaying_held_space() {
        let obstacles = crate::world::diagnostic_colliders();
        let start = crate::world::START_POSITION;
        let mut player = Player::new(start);
        let mut input = crate::input::ButtonInput::default();
        for _ in 0..3 {
            player.position.z = crate::world::GROUND.bounds().min.z - 2.0;
            input.sample(true);
            let mut returned = false;
            for _ in 0..180 {
                player.fixed_update(
                    Vec3::ZERO,
                    input.take(),
                    SpeedMode::Normal,
                    FixedClock::STEP as f32,
                    &obstacles,
                );
                if respawn_if_fallen(&mut player, start, &obstacles) {
                    input.pause(true);
                    returned = true;
                    break;
                }
            }
            assert!(returned);
            input.sample(true);
            player.fixed_update(
                Vec3::ZERO,
                input.take(),
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
            assert_eq!(player.position, start);
            input.sample(false);
            input.sample(true);
            player.fixed_update(
                Vec3::ZERO,
                input.take(),
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
            assert!(player.position.y > start.y);
        }
    }

    #[test]
    fn clearing_frame_time_discards_remainder_without_pausing_next_frame() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.frame_steps(0.01, true), None);
        assert_eq!(clock.frame_steps(0.01, true), Some(0));
        clock.clear_time();
        assert_eq!(clock.frame_steps(0.01, true), Some(0));
        assert_eq!(clock.frame_steps(0.01, true), Some(1));
    }

    #[test]
    fn falling_outside_scene_returns_once_at_different_frame_rates() {
        let obstacles = crate::world::diagnostic_colliders();
        for fps in [30, 60, 144] {
            let mut clock = FixedClock::default();
            let outside_ground = Vec3::new(
                crate::world::START_POSITION.x,
                0.0,
                crate::world::GROUND.bounds().min.z - 2.0,
            );
            let mut player = Player::new(outside_ground);
            let mut returns = 0;
            for _ in 0..fps * 3 {
                for _ in 0..clock.advance(1.0 / fps as f32) {
                    player.fixed_update(
                        Vec3::ZERO,
                        false,
                        SpeedMode::Normal,
                        FixedClock::STEP as f32,
                        &obstacles,
                    );
                    if respawn_if_fallen(&mut player, crate::world::START_POSITION, &obstacles) {
                        clock.clear_time();
                        returns += 1;
                        break;
                    }
                }
            }
            assert_eq!(returns, 1);
            assert_eq!(player.position, crate::world::START_POSITION);
            assert!(player.grounded);
        }
    }

    #[test]
    fn airborne_press_does_not_add_another_jump() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        let dt = FixedClock::STEP as f32;
        player.fixed_update(Vec3::ZERO, true, SpeedMode::Normal, dt, &obstacles);
        let velocity = player.vertical_velocity;
        assert!(velocity > 0.0);
        player.fixed_update(Vec3::ZERO, true, SpeedMode::Normal, dt, &obstacles);
        assert!((player.vertical_velocity - (velocity - GRAVITY * dt)).abs() < 0.00001);
    }

    #[test]
    fn holding_space_lands_without_repeating_and_release_allows_jump() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        let mut input = crate::input::ButtonInput::default();
        for _ in 0..180 {
            input.sample(true);
            player.fixed_update(
                Vec3::ZERO,
                input.take(),
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert_eq!(player.position.y, 0.0);
        assert!(player.grounded);
        input.sample(false);
        input.sample(true);
        player.fixed_update(
            Vec3::ZERO,
            input.take(),
            SpeedMode::Normal,
            FixedClock::STEP as f32,
            &obstacles,
        );
        assert!(player.position.y > 0.0);
        assert!(!player.grounded);
    }

    #[test]
    fn head_contact_stops_ascent_and_player_falls_back() {
        let ceiling = Aabb {
            min: Vec3::new(-2.0, 2.5, -2.0),
            max: Vec3::new(2.0, 2.6, 2.0),
        };
        let obstacles = [crate::world::GROUND.bounds(), ceiling];
        let mut player = Player::new(Vec3::ZERO);
        let mut touched = false;
        for step in 0..120 {
            player.fixed_update(
                Vec3::ZERO,
                step == 0,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
            assert!(player.position.y + collision::PLAYER_HEIGHT <= 2.50001);
            if player.position.y == 2.5 - collision::PLAYER_HEIGHT {
                touched = true;
                assert_eq!(player.vertical_velocity, 0.0);
                assert!(!player.grounded);
            }
        }
        assert!(touched);
        assert!(player.grounded);
        assert_eq!(player.position.y, 0.0);
    }

    #[test]
    fn jump_lands_on_low_platform() {
        let platform = Aabb {
            min: Vec3::new(2.0, 0.0, -2.0),
            max: Vec3::new(6.0, 0.5, 2.0),
        };
        let obstacles = [crate::world::GROUND.bounds(), platform];
        let mut player = Player::new(Vec3::ZERO);
        for step in 0..120 {
            let movement = if player.position.x < 3.0 {
                Vec3::X
            } else {
                Vec3::ZERO
            };
            player.fixed_update(
                movement,
                step == 0,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert!(player.position.x > 3.0);
        assert_eq!(player.position.y, 0.5);
        assert!(player.grounded);
    }

    #[test]
    fn jump_crosses_gap_with_horizontal_control_in_the_air() {
        let obstacles = [
            Aabb {
                min: Vec3::new(0.0, 0.0, -2.0),
                max: Vec3::new(4.0, 1.0, 2.0),
            },
            Aabb {
                min: Vec3::new(6.0, 0.0, -2.0),
                max: Vec3::new(10.0, 1.0, 2.0),
            },
        ];
        let mut player = Player::new(Vec3::new(3.0, 1.0, 0.0));
        for step in 0..120 {
            let movement = if player.position.x < 7.0 {
                Vec3::X
            } else {
                Vec3::ZERO
            };
            player.fixed_update(
                movement,
                step == 0,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert!(player.position.x > 7.0);
        assert_eq!(player.position.y, 1.0);
        assert!(player.grounded);
    }

    #[test]
    fn jump_height_and_landing_distance_match_at_different_frame_rates() {
        let mut reference: Option<(f32, f32)> = None;
        for fps in [30, 60, 144] {
            let mut player = Player::new(Vec3::ZERO);
            let mut clock = FixedClock::default();
            let mut input = crate::input::ButtonInput::default();
            let mut apex: f32 = 0.0;
            let mut landing = None;
            for _ in 0..fps * 2 {
                input.sample(true);
                for _ in 0..clock.advance(1.0 / fps as f32) {
                    player.fixed_update(
                        Vec3::X,
                        input.take(),
                        SpeedMode::Normal,
                        FixedClock::STEP as f32,
                        &[crate::world::GROUND.bounds()],
                    );
                    apex = apex.max(player.position.y);
                    if player.grounded && landing.is_none() {
                        landing = Some(player.position.x);
                    }
                }
            }
            let distance = landing.expect("прыжок заканчивается на земле в пределах двух секунд");
            assert!((apex - 4.0 / 3.0).abs() < 0.001);
            if let Some((height, length)) = reference {
                assert!((apex - height).abs() < 0.00001);
                assert!((distance - length).abs() < 0.00001);
            }
            reference = Some((apex, distance));
        }
    }

    #[test]
    fn stationary_player_lands_and_remains_on_the_ground() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::new(0.0, 5.0, 0.0));
        for _ in 0..600 {
            player.fixed_update(
                Vec3::ZERO,
                false,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert_eq!(player.position.y, 0.0);
        assert_eq!(player.vertical_velocity, 0.0);
        assert!(player.grounded);
    }

    #[test]
    fn scene_course_supports_sequential_descent_to_the_ground() {
        let obstacles = crate::world::diagnostic_colliders();
        let mut player = Player::new(Vec3::new(-4.0, 3.0, -14.0));
        for (target_x, height) in [(-1.0, 2.0), (4.0, 1.0), (9.0, 0.0)] {
            assert!(run_until(
                &mut player,
                Vec3::X,
                &obstacles,
                120,
                |player| player.position.x >= target_x
            ));
            run_steps(&mut player, Vec3::ZERO, 60, false, &obstacles);
            assert!((player.position.y - height).abs() < 0.00001);
            assert!(player.grounded);
        }
    }

    #[test]
    fn leaving_platform_loses_support_and_lands_on_lower_ground() {
        let platform = Aabb {
            min: Vec3::new(-1.0, 0.0, -1.0),
            max: Vec3::new(1.0, 2.0, 1.0),
        };
        let obstacles = [crate::world::GROUND.bounds(), platform];
        let mut player = Player::new(Vec3::new(0.0, 2.0, 0.0));
        assert!(run_until(&mut player, Vec3::X, &obstacles, 60, |player| {
            !player.grounded
        }));
        assert!(player.position.y < 2.0);
        for _ in 0..120 {
            player.fixed_update(
                Vec3::ZERO,
                false,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
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
            player.fixed_update(
                Vec3::X,
                false,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert_eq!(player.position.y, 0.0);
        assert!(player.grounded);
        assert!(player.position.x < 1.5);
    }

    fn travel_with_mode(fps: u32, speed_mode: SpeedMode) -> Player {
        let mut player = Player::new(Vec3::ZERO);
        let mut clock = FixedClock::default();
        for _ in 0..fps * 3 {
            for _ in 0..clock.advance(1.0 / fps as f32) {
                player.fixed_update(Vec3::X, false, speed_mode, FixedClock::STEP as f32, &[]);
            }
        }
        player
    }

    fn travel(fps: u32) -> Player {
        travel_with_mode(fps, SpeedMode::Normal)
    }

    #[test]
    fn movement_matches_at_different_render_rates() {
        let expected = travel_distance(3.0);
        let mut reference: Option<f32> = None;
        for fps in [30, 60, 144] {
            let distance = travel(fps).position.x;
            assert!((distance - expected).abs() < 0.1);
            if let Some(previous) = reference {
                assert!((distance - previous).abs() < 0.00001);
            }
            reference = Some(distance);
        }
    }

    #[test]
    fn precise_mode_reaches_limited_speed_without_exceeding_it() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..60 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
            assert!(player.horizontal_velocity.length() <= PRECISE_SPEED + 0.00001);
        }
        assert!((player.horizontal_velocity.length() - PRECISE_SPEED).abs() < 0.00001);
    }

    #[test]
    fn precise_mode_matches_at_different_render_rates() {
        let expected = travel_distance_for(PRECISE_SPEED, 3.0);
        let mut reference: Option<f32> = None;
        for fps in [30, 60, 144] {
            let distance = travel_with_mode(fps, SpeedMode::Precise).position.x;
            assert!((distance - expected).abs() < 0.1);
            if let Some(previous) = reference {
                assert!((distance - previous).abs() < 0.00001);
            }
            reference = Some(distance);
        }
    }

    #[test]
    fn switching_to_precise_mode_slows_and_release_restores_normal_speed() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        run_steps(&mut player, Vec3::X, 60, false, &obstacles);
        assert!((player.horizontal_velocity.length() - MOVE_SPEED).abs() < 0.00001);

        for _ in 0..60 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
        }
        assert!((player.horizontal_velocity.length() - PRECISE_SPEED).abs() < 0.0001);

        for _ in 0..60 {
            step(&mut player, Vec3::X, false, &obstacles);
        }
        assert!((player.horizontal_velocity.length() - MOVE_SPEED).abs() < 0.0001);
    }

    #[test]
    fn precise_mode_can_stop_at_platform_edge_without_falling() {
        let platform = Aabb {
            min: Vec3::new(-10.0, -1.0, -2.0),
            max: Vec3::new(10.0, 0.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];
        let mut player = Player::new(Vec3::new(5.0, 0.0, 0.0));
        for _ in 0..200 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
            if player.position.x >= 9.2 {
                break;
            }
        }
        assert!(player.position.x >= 9.2);
        for _ in 0..60 {
            step_with_mode(
                &mut player,
                Vec3::ZERO,
                false,
                SpeedMode::Precise,
                &obstacles,
            );
        }
        assert!(player.grounded);
        assert_eq!(player.position.y, 0.0);
        assert!(player.position.x <= 10.0 - foot + 0.00001);
        assert!(player.horizontal_velocity.length() < 0.00001);
    }

    #[test]
    fn precise_mode_turns_within_a_narrow_platform() {
        let platform = Aabb {
            min: Vec3::new(-2.0, -1.0, -2.0),
            max: Vec3::new(2.0, 0.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..20 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
        }
        for _ in 0..20 {
            step_with_mode(&mut player, Vec3::Z, false, SpeedMode::Precise, &obstacles);
        }
        assert!(player.grounded);
        assert!(player.position.x.abs() <= 2.0 - foot);
        assert!(player.position.z.abs() <= 2.0 - foot);
        assert!(player.horizontal_velocity.z > 2.0);
        assert!(player.horizontal_velocity.x.abs() < 0.5);
    }

    #[test]
    fn precise_jump_has_shorter_range() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..60 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
        }
        let takeoff_x = player.position.x;
        step_with_mode(&mut player, Vec3::X, true, SpeedMode::Precise, &obstacles);
        let mut steps = 0;
        while !player.grounded && steps < 120 {
            step_with_mode(&mut player, Vec3::X, false, SpeedMode::Precise, &obstacles);
            steps += 1;
        }
        assert!(player.grounded);
        assert!((player.position.x - takeoff_x - 2.0).abs() < 0.2);
    }

    #[test]
    fn focus_pause_freezes_player_and_discards_partial_step() {
        let mut clock = FixedClock::default();
        let mut player = Player::new(Vec3::ZERO);
        assert_eq!(clock.frame_steps(0.02, true), None);
        assert_eq!(clock.frame_steps(0.01, true), Some(0));
        for (dt, focused) in [(0.02, false), (10.0, false), (0.2, true)] {
            for _ in 0..clock.frame_steps(dt, focused).unwrap_or(0) {
                player.fixed_update(
                    Vec3::X,
                    false,
                    SpeedMode::Normal,
                    FixedClock::STEP as f32,
                    &[],
                );
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
                    player.fixed_update(
                        Vec3::X + Vec3::Z,
                        false,
                        SpeedMode::Normal,
                        FixedClock::STEP as f32,
                        &obstacles,
                    );
                }
            }
            assert!((player.position.x - 1.5).abs() < 0.001);
            assert!((player.position.z - travel_distance(3.0) / 2.0_f32.sqrt()).abs() < 0.1);
            run_steps(&mut player, -Vec3::X, 10, false, &obstacles);
            assert!(player.position.x < 1.4);
        }
    }

    #[test]
    fn diagonal_does_not_increase_speed() {
        let mut straight = Player::new(Vec3::ZERO);
        let mut diagonal = Player::new(Vec3::ZERO);
        straight.fixed_update(
            Vec3::X,
            false,
            SpeedMode::Normal,
            FixedClock::STEP as f32,
            &[],
        );
        diagonal.fixed_update(
            Vec3::X + Vec3::Z,
            false,
            SpeedMode::Normal,
            FixedClock::STEP as f32,
            &[],
        );
        assert!((straight.position.length() - diagonal.position.length()).abs() < 0.00001);
    }

    #[test]
    fn ground_acceleration_reaches_max_speed_without_exceeding_it() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..60 {
            step(&mut player, Vec3::X, false, &obstacles);
            assert!(player.horizontal_velocity.length() <= MOVE_SPEED + 0.00001);
        }
        assert!((player.horizontal_velocity.length() - MOVE_SPEED).abs() < 0.00001);
    }

    #[test]
    fn releasing_input_brakes_to_a_stop_within_expected_distance() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        run_steps(&mut player, Vec3::X, 60, false, &obstacles);
        let start = player.position.x;
        let mut braking_steps = 0;
        while player.horizontal_velocity.length() > 0.00001 && braking_steps < 60 {
            step(&mut player, Vec3::ZERO, false, &obstacles);
            braking_steps += 1;
        }
        let distance = player.position.x - start;
        assert!(braking_steps < 15);
        assert!((0.3..0.6).contains(&distance));
        assert!(player.grounded);
    }

    #[test]
    fn player_can_stop_at_platform_edge_without_falling() {
        let platform = Aabb {
            min: Vec3::new(-10.0, -1.0, -2.0),
            max: Vec3::new(10.0, 0.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];
        let mut player = Player::new(Vec3::new(-8.0, 0.0, 0.0));
        assert!(run_until(
            &mut player,
            Vec3::X,
            &obstacles,
            200,
            |player| player.position.x >= 9.2
        ));
        run_steps(&mut player, Vec3::ZERO, 60, false, &obstacles);
        assert!(player.grounded);
        assert_eq!(player.position.y, 0.0);
        assert!(player.position.x <= 10.0 - foot + 0.00001);
        assert!(player.horizontal_velocity.length() < 0.00001);
    }

    #[test]
    fn player_falls_when_center_passes_foot_support_at_edge() {
        let platform = Aabb {
            min: Vec3::new(-10.0, -1.0, -2.0),
            max: Vec3::new(10.0, 0.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];

        let mut supported = Player::new(Vec3::new(10.0 - foot - 0.05, 0.0, 0.0));
        step(&mut supported, Vec3::ZERO, false, &obstacles);
        assert!(supported.grounded);

        let mut overhanging = Player::new(Vec3::new(10.0 - foot + 0.05, 0.0, 0.0));
        step(&mut overhanging, Vec3::ZERO, false, &obstacles);
        assert!(!overhanging.grounded);
        assert!(overhanging.position.y < 0.0);
    }

    #[test]
    fn walking_past_the_edge_starts_falling_once_feet_leave_the_platform() {
        let platform = Aabb {
            min: Vec3::new(-10.0, -1.0, -2.0),
            max: Vec3::new(10.0, 0.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];
        let mut player = Player::new(Vec3::new(10.0 - foot - 0.05, 0.0, 0.0));
        player.horizontal_velocity = Vec3::X * MOVE_SPEED;
        for _ in 0..5 {
            step(&mut player, Vec3::X, false, &obstacles);
        }
        assert!(!player.grounded);
        assert!(player.position.y < 0.0);
    }

    #[test]
    fn partial_foot_landing_stays_on_the_platform() {
        let platform = Aabb {
            min: Vec3::new(0.0, 0.0, -2.0),
            max: Vec3::new(10.0, 1.0, 2.0),
        };
        let foot = crate::collision::FOOT_SUPPORT_HALF_WIDTH;
        let obstacles = [platform];
        let partial_x = 10.0 + foot - 0.05;
        let mut landed = Player::new(Vec3::new(partial_x, 2.0, 0.0));
        run_steps(&mut landed, Vec3::ZERO, 30, false, &obstacles);
        assert!(landed.grounded);
        assert_eq!(landed.position.y, 1.0);

        step(&mut landed, Vec3::ZERO, false, &obstacles);
        assert!(landed.grounded);
        assert_eq!(landed.position.y, 1.0);

        for _ in 0..10 {
            step(&mut landed, Vec3::X, false, &obstacles);
        }
        assert!(!landed.grounded);

        let mut outside = Player::new(Vec3::new(10.0 + foot + 0.05, 2.0, 0.0));
        run_steps(&mut outside, Vec3::ZERO, 30, false, &obstacles);
        assert!(!outside.grounded);
        assert!(outside.position.y < 0.5);
    }

    #[test]
    fn air_control_can_reverse_direction_and_preserves_momentum_without_input() {
        let mut reversing = Player::new(Vec3::Y * 10.0);
        reversing.horizontal_velocity = Vec3::X * MOVE_SPEED;
        for _ in 0..30 {
            step(&mut reversing, -Vec3::X, false, &[]);
        }
        assert!(reversing.horizontal_velocity.x < 0.0);
        assert!(!reversing.grounded);

        let mut drifting = Player::new(Vec3::Y * 10.0);
        drifting.horizontal_velocity = Vec3::X * MOVE_SPEED;
        step(&mut drifting, Vec3::ZERO, false, &[]);
        assert!((drifting.horizontal_velocity.x - MOVE_SPEED).abs() < 0.00001);
    }

    #[test]
    fn landing_with_residual_speed_stops_inside_the_platform() {
        let ground = Aabb {
            min: Vec3::new(-10.0, -1.0, -2.0),
            max: Vec3::new(5.0, 0.0, 2.0),
        };
        let platform = Aabb {
            min: Vec3::new(7.0, 0.0, -2.0),
            max: Vec3::new(11.0, 1.0, 2.0),
        };
        let obstacles = [ground, platform];
        let mut player = Player::new(Vec3::new(3.0, 0.0, 0.0));
        assert!(run_until(
            &mut player,
            Vec3::X,
            &obstacles,
            120,
            |player| player.position.x >= 4.5
        ));
        assert!(jump_until_landed(&mut player, Vec3::X, &obstacles, 120));
        assert!(player.grounded);
        assert_eq!(player.position.y, 1.0);
        let landing_x = player.position.x;
        run_steps(&mut player, Vec3::ZERO, 60, false, &obstacles);
        assert!(player.grounded);
        assert!(player.position.x < 11.5);
        assert!((player.position.x - landing_x).abs() < 0.6);
        assert!(player.horizontal_velocity.length() < 0.00001);
    }

    #[test]
    fn full_speed_jump_has_reference_height_and_distance() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        run_steps(&mut player, Vec3::X, 60, false, &obstacles);
        let takeoff_x = player.position.x;
        step(&mut player, Vec3::X, true, &obstacles);
        let mut apex = player.position.y;
        let mut steps = 0;
        while !player.grounded && steps < 120 {
            step(&mut player, Vec3::X, false, &obstacles);
            apex = apex.max(player.position.y);
            steps += 1;
        }
        assert!(player.grounded);
        assert!((apex - 4.0 / 3.0).abs() < 0.01);
        assert!((player.position.x - takeoff_x - 4.6667).abs() < 0.2);
    }

    #[test]
    fn held_space_does_not_repeat_after_a_moving_landing() {
        let obstacles = [crate::world::GROUND.bounds()];
        let mut player = Player::new(Vec3::ZERO);
        let mut input = crate::input::ButtonInput::default();
        run_steps(&mut player, Vec3::X, 20, false, &obstacles);
        input.sample(true);
        step(&mut player, Vec3::X, input.take(), &obstacles);
        let mut landed = false;
        for _ in 0..120 {
            input.sample(true);
            step(&mut player, Vec3::X, input.take(), &obstacles);
            if player.grounded {
                landed = true;
                break;
            }
        }
        assert!(landed);
        input.sample(true);
        step(&mut player, Vec3::ZERO, input.take(), &obstacles);
        assert!(player.grounded);
        assert_eq!(player.vertical_velocity, 0.0);
        input.sample(false);
        input.sample(true);
        step(&mut player, Vec3::ZERO, input.take(), &obstacles);
        assert!(player.vertical_velocity > 0.0);
        assert!(!player.grounded);
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
