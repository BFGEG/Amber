use bfge::math::Vec3;

use crate::{
    camera::FollowCamera,
    input::ButtonInput,
    pause::PauseState,
    recovery::{CheckpointPose, Recovery},
    section::SectionState,
    simulation::{FixedClock, Player},
    world::{self, Aabb},
    zone::{Zone, ZoneId, ZoneSet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Course,
    Physics,
    Demo,
}

impl SceneKind {
    pub fn event_zones(self) -> Vec<Zone> {
        if self == Self::Course {
            world::event_zones().to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn checkpoint_zones(self) -> Vec<Zone> {
        if self == Self::Course {
            vec![Zone::one_shot(world::CHECKPOINT_ZONE)]
        } else {
            Vec::new()
        }
    }

    pub fn hazard_zones(self) -> Vec<Zone> {
        if self == Self::Course {
            world::hazard_zones().to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn section_zone(self) -> Option<Aabb> {
        (self == Self::Course).then_some(world::SECTION_EVENT_ZONE)
    }
}

/// Состояние одной загрузки сцены. Графические ID и цикл BFGE остаются у приложения.
pub struct SceneState {
    pub kind: SceneKind,
    pub timing_demo: bool,
    pub start_position: Vec3,
    pub initial_direction: Vec3,
    pub player: Player,
    pub colliders: Vec<Aabb>,
    pub recovery: Recovery,
    pub section: SectionState,
    pub zone_set: ZoneSet,
    pub checkpoint_set: ZoneSet,
    pub hazard_set: ZoneSet,
    pub follow_camera: FollowCamera,
    pub yaw: f32,
    pub pitch: f32,
    pub clock: FixedClock,
    pub pause: PauseState,
    pub jump_input: ButtonInput,
    pub return_input: ButtonInput,
    exit_input: ButtonInput,
    pub debug_pressed: bool,
    pub hazard_hold: f32,
    pub hazard_alert: Option<ZoneId>,
    pub demo_steps: usize,
    pub demo_frames: u32,
    pub demo_elapsed: f64,
}

impl SceneState {
    pub fn new(kind: SceneKind, timing_demo: bool) -> Self {
        let timing_demo = timing_demo && kind == SceneKind::Physics;
        let start_position = if kind == SceneKind::Demo {
            Vec3::ZERO
        } else if timing_demo {
            world::TIMING_START_POSITION
        } else {
            world::START_POSITION
        };
        let initial_direction = if timing_demo {
            Vec3::X
        } else {
            Vec3::new(-1.0, -0.46, 0.0)
        }
        .normalize();
        let yaw = initial_direction.x.atan2(-initial_direction.z);
        let pitch = initial_direction.y.asin();
        Self {
            kind,
            timing_demo,
            start_position,
            initial_direction,
            player: Player::new(start_position),
            colliders: match kind {
                SceneKind::Course => world::course_colliders(),
                SceneKind::Physics => world::diagnostic_colliders(),
                SceneKind::Demo => vec![world::DEMO_FLOOR.bounds()],
            },
            recovery: Recovery::new(CheckpointPose {
                position: start_position,
                yaw,
                pitch,
            }),
            section: SectionState::new(kind.section_zone()),
            zone_set: ZoneSet::new(kind.event_zones()),
            checkpoint_set: ZoneSet::new(kind.checkpoint_zones()),
            hazard_set: ZoneSet::new(kind.hazard_zones()),
            follow_camera: FollowCamera::new(start_position),
            yaw,
            pitch,
            clock: FixedClock::default(),
            pause: PauseState::default(),
            jump_input: ButtonInput::default(),
            return_input: ButtonInput::default(),
            exit_input: ButtonInput::default(),
            debug_pressed: false,
            hazard_hold: 0.0,
            hazard_alert: None,
            demo_steps: 0,
            demo_frames: 0,
            demo_elapsed: 0.0,
        }
    }

    pub fn exit_available(&self) -> bool {
        self.kind == SceneKind::Course
            && world::exit_contains(crate::collision::player_bounds(self.player.position).center())
    }

    pub fn exit_request(&mut self, can_interact: bool, held: bool) -> Option<SceneKind> {
        if !can_interact {
            self.exit_input.pause(held);
            return None;
        }
        self.exit_input.sample(held);
        (self.exit_input.take() && self.exit_available()).then_some(SceneKind::Demo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn course_exit_requires_a_new_press_inside_the_circle() {
        let mut state = SceneState::new(SceneKind::Course, false);
        assert_eq!(state.exit_request(true, true), None);
        state.player.position = world::EXIT_CENTER + Vec3::new(1.8, 0.0, 1.8);
        assert!(!state.exit_available());
        state.player.position = world::EXIT_CENTER;
        assert!(state.exit_available());
        assert_eq!(state.exit_request(true, true), None);
        state.exit_request(true, false);
        state.player.position = world::EXIT_CENTER + Vec3::X * 3.0;
        assert!(!state.exit_available());
        assert_eq!(state.exit_request(true, true), None);
        state.player.position = world::EXIT_CENTER;
        assert_eq!(state.exit_request(false, true), None);
        assert_eq!(state.exit_request(true, true), None);
        state.exit_request(true, false);
        assert_eq!(state.exit_request(true, true), Some(SceneKind::Demo));
        assert_eq!(state.exit_request(true, true), None);

        let mut demo = SceneState::new(SceneKind::Demo, false);
        demo.player.position = world::EXIT_CENTER;
        assert_eq!(demo.exit_request(true, true), None);
    }

    #[test]
    fn round_trips_drop_old_checkpoint_events_pause_and_section_changes() {
        let mut loaded = SceneState::new(SceneKind::Course, false);
        for _ in 0..20 {
            loaded.recovery.activate(CheckpointPose {
                position: world::CHECKPOINT_POSITION,
                yaw: 1.0,
                pitch: -0.3,
            });
            loaded.section.update(world::SECTION_EVENT_ZONE.center());
            assert!(loaded.section.changed);
            loaded
                .checkpoint_set
                .update(crate::collision::player_bounds(world::CHECKPOINT_POSITION).center());
            loaded
                .pause
                .frame_steps(&mut loaded.clock, 0.02, true, false);
            loaded
                .pause
                .frame_steps(&mut loaded.clock, 0.02, true, true);
            assert!(loaded.pause.is_paused());
            loaded.hazard_hold = 0.15;
            loaded.hazard_alert = Some(ZoneId(0));
            loaded.return_input.sample(true);

            loaded = SceneState::new(SceneKind::Demo, true);
            assert_eq!(loaded.player.position, Vec3::ZERO);
            assert_eq!(loaded.recovery.pose().position, Vec3::ZERO);
            assert!(!loaded.timing_demo);
            assert!(!loaded.pause.is_paused());
            assert_eq!(loaded.hazard_hold, 0.0);
            assert!(!loaded.return_input.take());
            assert!(
                loaded
                    .checkpoint_set
                    .update(world::CHECKPOINT_ZONE.center())
                    .is_empty()
            );
            assert!(
                loaded
                    .hazard_set
                    .update(world::HAZARD_AFTER_ASCENT[0].center())
                    .is_empty()
            );
            assert!(crate::collision::has_support(
                loaded.player.position,
                &loaded.colliders
            ));

            loaded = SceneState::new(SceneKind::Course, false);
            assert_eq!(loaded.recovery.pose().position, world::START_POSITION);
            assert!(!loaded.section.changed);
            assert!(!loaded.pause.is_paused());
            assert!(
                loaded
                    .checkpoint_set
                    .update(crate::collision::player_bounds(world::CHECKPOINT_POSITION).center())
                    .contains(&crate::zone::ZoneEvent::Triggered(ZoneId(0)))
            );
        }
    }

    #[test]
    fn reload_of_each_scene_starts_grounded_after_one_simulation_step() {
        for kind in [SceneKind::Course, SceneKind::Physics, SceneKind::Demo] {
            let mut loaded = SceneState::new(kind, false);
            loaded.player.fixed_update(
                Vec3::ZERO,
                false,
                crate::simulation::SpeedMode::Normal,
                FixedClock::STEP as f32,
                &loaded.colliders,
            );
            assert!(loaded.player.grounded);
            assert_eq!(loaded.player.position, loaded.start_position);
        }
    }

    #[test]
    fn new_load_has_no_progress_pending_input_or_hazard_delay() {
        let mut previous = SceneState::new(SceneKind::Course, false);
        previous.recovery.activate(CheckpointPose {
            position: world::CHECKPOINT_POSITION,
            yaw: 1.0,
            pitch: -0.3,
        });
        previous.section.update(world::SECTION_EVENT_ZONE.center());
        previous.jump_input.sample(true);
        previous.return_input.sample(true);
        previous.hazard_hold = 0.15;
        previous.hazard_alert = Some(ZoneId(0));
        let mut loaded = SceneState::new(SceneKind::Course, false);
        assert_eq!(loaded.recovery.pose().position, world::START_POSITION);
        assert!(!loaded.section.changed);
        assert!(!loaded.jump_input.take());
        assert!(!loaded.return_input.take());
        assert_eq!(loaded.hazard_hold, 0.0);
        assert_eq!(loaded.hazard_alert, None);
        assert!(!loaded.pause.is_paused());
        assert_eq!(loaded.player.position, world::START_POSITION);
        assert_eq!(loaded.player.horizontal_velocity, Vec3::ZERO);
        assert_eq!(loaded.player.vertical_velocity, 0.0);
    }

    #[test]
    fn diagnostic_scene_does_not_run_course_zones() {
        let mut loaded = SceneState::new(SceneKind::Physics, true);
        assert!(
            loaded
                .zone_set
                .update(world::TUTORIAL_EVENT_ZONE.center())
                .is_empty()
        );
        assert!(
            loaded
                .checkpoint_set
                .update(world::CHECKPOINT_ZONE.center())
                .is_empty()
        );
        assert!(
            loaded
                .hazard_set
                .update(world::HAZARD_AFTER_ASCENT[0].center())
                .is_empty()
        );
        loaded.section.update(world::SECTION_EVENT_ZONE.center());
        assert!(!loaded.section.changed);
        assert_eq!(
            loaded.recovery.pose().position,
            world::TIMING_START_POSITION
        );
    }
}
