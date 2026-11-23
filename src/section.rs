use bfge::math::Vec3;

use crate::world::Aabb;
use crate::zone::{Zone, ZoneEvent, ZoneSet};

/// Изменяемое состояние текущего участка; не зависит от ID графической сцены.
pub struct SectionState {
    zones: ZoneSet,
    pub changed: bool,
}

impl SectionState {
    pub fn new(demo_zone: Option<Aabb>) -> Self {
        Self {
            zones: ZoneSet::new(demo_zone.into_iter().map(Zone::one_shot)),
            changed: false,
        }
    }

    pub fn update(&mut self, player_center: Vec3) {
        if self
            .zones
            .update(player_center)
            .iter()
            .any(|event| matches!(event, ZoneEvent::Triggered(_)))
        {
            self.changed = true;
        }
    }

    pub fn reset_at(&mut self, player_center: Vec3) {
        self.changed = false;
        self.zones.reset_at(player_center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world;

    #[test]
    fn changed_object_resets_and_can_be_triggered_on_the_next_attempt() {
        let zone = world::SECTION_EVENT_ZONE;
        let mut section = SectionState::new(Some(zone));
        section.update(zone.center());
        assert!(section.changed);
        section.reset_at(world::START_POSITION);
        assert!(!section.changed);
        section.update(zone.center());
        assert!(section.changed);
    }

    #[test]
    fn spawning_inside_zone_does_not_replay_its_one_shot_action() {
        let zone = world::SECTION_EVENT_ZONE;
        let mut section = SectionState::new(Some(zone));
        section.update(zone.center());
        section.reset_at(zone.center());
        for _ in 0..60 {
            section.update(zone.center());
            assert!(!section.changed);
        }
        section.update(world::START_POSITION);
        section.update(zone.center());
        assert!(section.changed);
    }
}
