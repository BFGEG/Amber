use bfge::math::Vec3;

use crate::{collision, section::SectionState, simulation::Player, world::Aabb};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckpointPose {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

/// Общая точка восстановления для падений и опасных зон текущей сцены.
pub struct Recovery {
    start: CheckpointPose,
    checkpoint: Option<CheckpointPose>,
}

impl Recovery {
    pub fn new(start: CheckpointPose) -> Self {
        Self {
            start,
            checkpoint: None,
        }
    }

    pub fn pose(&self) -> CheckpointPose {
        self.checkpoint.unwrap_or(self.start)
    }

    /// Единый порядок для падения, опасности и R: объекты участка, затем персонаж.
    /// Камера и графическая сцена синхронизируются вызывающим кодом в этом же кадре.
    pub fn restore(
        &self,
        section: &mut SectionState,
        player: &mut Player,
        obstacles: &[Aabb],
    ) -> CheckpointPose {
        let pose = self.pose();
        section.reset_at(collision::player_bounds(pose.position).center());
        player.respawn(pose.position, obstacles);
        pose
    }

    /// В этой сцене одна контрольная точка: повторный вход не меняет её ракурс.
    pub fn activate(&mut self, pose: CheckpointPose) -> bool {
        if self.checkpoint.is_some() {
            false
        } else {
            self.checkpoint = Some(pose);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{collision, simulation, world, zone};

    fn start() -> CheckpointPose {
        CheckpointPose {
            position: world::START_POSITION,
            yaw: -1.5,
            pitch: -0.4,
        }
    }

    fn saved() -> CheckpointPose {
        CheckpointPose {
            position: world::CHECKPOINT_POSITION,
            yaw: 0.7,
            pitch: -0.3,
        }
    }

    #[test]
    fn restore_resets_changed_objects_and_airborne_player_to_start_or_checkpoint() {
        let obstacles = world::course_colliders();
        for checkpoint in [false, true] {
            let mut recovery = Recovery::new(start());
            if checkpoint {
                recovery.activate(saved());
            }
            let mut section = SectionState::new(Some(world::SECTION_EVENT_ZONE));
            section.update(world::SECTION_EVENT_ZONE.center());
            assert!(section.changed);
            let mut player = simulation::Player::new(start().position);
            player.fixed_update(
                Vec3::X,
                true,
                simulation::SpeedMode::Normal,
                1.0 / 60.0,
                &obstacles,
            );
            assert!(player.vertical_velocity > 0.0);
            let pose = recovery.restore(&mut section, &mut player, &obstacles);
            assert!(!section.changed);
            assert_eq!(pose, if checkpoint { saved() } else { start() });
            assert_eq!(player.position, pose.position);
            assert_eq!(player.horizontal_velocity, Vec3::ZERO);
            assert_eq!(player.vertical_velocity, 0.0);
            assert!(player.grounded);
        }
    }

    #[test]
    fn held_return_button_restores_once_at_different_frame_rates() {
        let obstacles = world::course_colliders();
        for fps in [30, 60, 144] {
            let mut recovery = Recovery::new(start());
            recovery.activate(saved());
            let mut section = SectionState::new(Some(world::SECTION_EVENT_ZONE));
            section.update(world::SECTION_EVENT_ZONE.center());
            let mut player = simulation::Player::new(start().position + Vec3::Y);
            let mut button = crate::input::ButtonInput::default();
            let mut restores = 0;
            for _ in 0..fps {
                button.sample(true);
                if button.take() {
                    recovery.restore(&mut section, &mut player, &obstacles);
                    restores += 1;
                }
            }
            assert_eq!(restores, 1);
            assert_eq!(player.position, saved().position);
            assert!(!section.changed);
            section.update(world::SECTION_EVENT_ZONE.center());
            assert!(section.changed);
            button.sample(false);
            button.sample(true);
            assert!(button.take());
            recovery.restore(&mut section, &mut player, &obstacles);
            assert!(!section.changed);
            assert_eq!(recovery.pose(), saved());
        }
    }

    #[test]
    fn before_activation_respawn_is_the_original_start() {
        assert_eq!(Recovery::new(start()).pose(), start());
    }

    #[test]
    fn activation_snapshots_pose_and_repeated_calls_do_not_replace_it() {
        let mut recovery = Recovery::new(start());
        assert!(recovery.activate(saved()));
        assert!(!recovery.activate(CheckpointPose {
            yaw: -2.0,
            pitch: -0.8,
            ..start()
        }));
        assert_eq!(recovery.pose(), saved());
    }

    #[test]
    fn separate_zone_activates_once_and_return_does_not_reactivate_it() {
        let mut recovery = Recovery::new(start());
        let mut zones = zone::ZoneSet::new([zone::Zone::one_shot(world::CHECKPOINT_ZONE)]);
        let inside = collision::player_bounds(world::CHECKPOINT_POSITION).center();
        let triggered = zone::ZoneEvent::Triggered(zone::ZoneId(0));
        assert!(zones.update(inside).contains(&triggered));
        assert!(recovery.activate(saved()));
        for position in [inside, start().position, inside] {
            assert!(!zones.update(position).contains(&triggered));
            assert_eq!(recovery.pose(), saved());
        }
    }

    #[test]
    fn checkpoint_has_full_support_and_is_outside_solids_and_hazards() {
        let position = world::CHECKPOINT_POSITION;
        let player = collision::player_bounds(position);
        assert!(world::CHECKPOINT_ZONE.contains(player.center()));
        let obstacles = world::course_colliders();
        assert!(collision::has_support(position, &obstacles));
        for obstacle in obstacles {
            assert!(!(player.min.cmplt(obstacle.max) & player.max.cmpgt(obstacle.min)).all());
        }
        for hazard in world::hazard_zones() {
            assert!(!hazard.bounds.contains(position));
        }
        assert!(world::HAZARD_AFTER_ASCENT[0].max.x < world::CHECKPOINT_ZONE.min.x);
    }

    #[test]
    fn both_fall_and_hazard_restore_the_active_pose_with_zero_velocity() {
        let obstacles = world::course_colliders();
        let mut recovery = Recovery::new(start());
        let mut player = simulation::Player::new(Vec3::new(200.0, -20.0, 0.0));
        assert!(player.is_below_fall_limit());
        recovery.restore(&mut SectionState::new(None), &mut player, &obstacles);
        assert_eq!(player.position, start().position);
        assert!(recovery.activate(saved()));
        for _ in 0..3 {
            player.position = Vec3::new(200.0, -20.0, 0.0);
            player.vertical_velocity = -20.0;
            assert!(player.is_below_fall_limit());
            recovery.restore(&mut SectionState::new(None), &mut player, &obstacles);
            assert_eq!(player.position, world::CHECKPOINT_POSITION);
            let mut hazards = zone::ZoneSet::new(world::hazard_zones());
            player.position = world::HAZARD_AFTER_ASCENT[0].center();
            assert!(
                hazards
                    .update(player.position)
                    .iter()
                    .any(|event| matches!(event, zone::ZoneEvent::Entered(_)))
            );
            player.horizontal_velocity = Vec3::X * 7.0;
            player.vertical_velocity = -20.0;
            recovery.restore(&mut SectionState::new(None), &mut player, &obstacles);
            assert_eq!(player.position, world::CHECKPOINT_POSITION);
            assert_eq!(player.horizontal_velocity, Vec3::ZERO);
            assert_eq!(player.vertical_velocity, 0.0);
            assert!(player.grounded);
            assert!(
                hazards
                    .update(player.position)
                    .iter()
                    .all(|event| matches!(event, zone::ZoneEvent::Exited(_)))
            );
        }
    }
}
