use crate::{input::ButtonInput, simulation::FixedClock};

/// Ручная пауза сохраняется после потери фокуса до явного продолжения по Esc.
#[derive(Default)]
pub struct PauseState {
    paused: bool,
    was_focused: bool,
    escape: ButtonInput,
}

impl PauseState {
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn frame_steps(
        &mut self,
        clock: &mut FixedClock,
        dt: f32,
        focused: bool,
        escape_held: bool,
    ) -> Option<usize> {
        if !focused {
            self.paused |= self.was_focused;
            self.escape.pause(escape_held);
        } else if !self.was_focused {
            // Удержанная при возврате в окно клавиша не должна снимать паузу.
            self.escape.pause(escape_held);
        } else {
            self.escape.sample(escape_held);
            if self.escape.take() {
                self.paused = !self.paused;
            }
        }
        self.was_focused = focused;
        clock.frame_steps(dt, focused && !self.paused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_toggles_once_per_press_and_resume_discards_elapsed_time() {
        let mut pause = PauseState::default();
        let mut clock = FixedClock::default();
        assert_eq!(pause.frame_steps(&mut clock, 0.02, true, false), None);
        assert_eq!(pause.frame_steps(&mut clock, 0.02, true, true), None);
        assert!(pause.is_paused());
        assert_eq!(pause.frame_steps(&mut clock, 10.0, true, true), None);
        assert!(pause.is_paused());
        pause.frame_steps(&mut clock, 0.02, true, false);
        assert_eq!(pause.frame_steps(&mut clock, 10.0, true, true), None);
        assert!(!pause.is_paused());
        assert_eq!(
            pause.frame_steps(&mut clock, 1.0 / 60.0, true, true),
            Some(1)
        );
    }

    #[test]
    fn focus_loss_requires_deliberate_resume_and_ignores_held_escape() {
        let mut pause = PauseState::default();
        let mut clock = FixedClock::default();
        pause.frame_steps(&mut clock, 0.02, true, false);
        pause.frame_steps(&mut clock, 0.02, false, false);
        assert!(pause.is_paused());
        assert_eq!(pause.frame_steps(&mut clock, 5.0, true, true), None);
        assert!(pause.is_paused());
        pause.frame_steps(&mut clock, 0.02, true, false);
        pause.frame_steps(&mut clock, 0.02, true, true);
        assert!(!pause.is_paused());
    }

    #[test]
    fn initial_unfocused_window_does_not_require_extra_resume() {
        let mut pause = PauseState::default();
        let mut clock = FixedClock::default();
        assert_eq!(pause.frame_steps(&mut clock, 10.0, false, false), None);
        assert!(!pause.is_paused());
        assert_eq!(pause.frame_steps(&mut clock, 10.0, true, false), None);
        assert_eq!(
            pause.frame_steps(&mut clock, 1.0 / 60.0, true, false),
            Some(1)
        );
    }

    #[test]
    fn jump_and_section_timer_freeze_during_pause_at_different_frame_rates() {
        use crate::{
            simulation::{Player, SpeedMode},
            world,
        };
        use bfge::math::Vec3;

        for fps in [30, 60, 144] {
            let mut pause = PauseState::default();
            let mut clock = FixedClock::default();
            let mut player = Player::new(world::START_POSITION);
            let obstacles = world::course_colliders();
            player.fixed_update(
                Vec3::ZERO,
                true,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
            let position = player.position;
            let velocity = player.vertical_velocity;
            let mut timer = 0.15_f32;
            pause.frame_steps(&mut clock, 0.02, true, false);
            for _ in 0..fps * 3 {
                if let Some(steps) = pause.frame_steps(&mut clock, 1.0 / fps as f32, true, true) {
                    timer -= 1.0 / fps as f32;
                    for _ in 0..steps {
                        player.fixed_update(
                            Vec3::ZERO,
                            false,
                            SpeedMode::Normal,
                            FixedClock::STEP as f32,
                            &obstacles,
                        );
                    }
                }
            }
            assert_eq!(player.position, position);
            assert_eq!(player.vertical_velocity, velocity);
            assert_eq!(timer, 0.15);
            pause.frame_steps(&mut clock, 0.02, true, false);
            assert_eq!(pause.frame_steps(&mut clock, 5.0, true, true), None);
            let steps = pause
                .frame_steps(&mut clock, 1.0 / 60.0, true, false)
                .expect("первый обычный кадр после продолжения разрешён");
            for _ in 0..steps {
                player.fixed_update(
                    Vec3::ZERO,
                    false,
                    SpeedMode::Normal,
                    FixedClock::STEP as f32,
                    &obstacles,
                );
            }
            assert!(player.position.y > position.y);
        }
    }

    #[test]
    fn resumed_jump_can_be_cancelled_by_manual_checkpoint_recovery() {
        use crate::{
            recovery::{CheckpointPose, Recovery},
            section::SectionState,
            simulation::{Player, SpeedMode},
            world,
        };
        use bfge::math::Vec3;

        let mut recovery = Recovery::new(CheckpointPose {
            position: world::START_POSITION,
            yaw: 0.0,
            pitch: -0.4,
        });
        let saved = CheckpointPose {
            position: world::CHECKPOINT_POSITION,
            yaw: 1.0,
            pitch: -0.3,
        };
        recovery.activate(saved);
        let mut section = SectionState::new(Some(world::SECTION_EVENT_ZONE));
        section.update(world::SECTION_EVENT_ZONE.center());
        let obstacles = world::course_colliders();
        let mut player = Player::new(saved.position);
        player.fixed_update(
            Vec3::ZERO,
            true,
            SpeedMode::Normal,
            FixedClock::STEP as f32,
            &obstacles,
        );
        assert!(!player.grounded);

        let mut pause = PauseState::default();
        let mut clock = FixedClock::default();
        let mut return_input = ButtonInput::default();
        pause.frame_steps(&mut clock, 0.02, true, false);
        assert_eq!(pause.frame_steps(&mut clock, 0.02, true, true), None);
        return_input.pause(true);
        pause.frame_steps(&mut clock, 1.0, true, false);
        assert_eq!(pause.frame_steps(&mut clock, 1.0, true, true), None);
        return_input.sample(true);
        assert!(!return_input.take());
        let steps = pause
            .frame_steps(&mut clock, 1.0 / 60.0, true, false)
            .expect("после продолжения разрешён один обычный шаг");
        for _ in 0..steps {
            player.fixed_update(
                Vec3::ZERO,
                false,
                SpeedMode::Normal,
                FixedClock::STEP as f32,
                &obstacles,
            );
        }
        assert!(player.position.y > saved.position.y);
        return_input.sample(false);
        return_input.sample(true);
        assert!(return_input.take());
        assert_eq!(
            recovery.restore(&mut section, &mut player, &obstacles),
            saved
        );
        assert_eq!(player.position, saved.position);
        assert_eq!(player.vertical_velocity, 0.0);
        assert!(player.grounded);
        assert!(!section.changed);
    }
}
