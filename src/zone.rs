use bfge::math::Vec3;

use crate::world::Aabb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneMode {
    Repeating,
    OneShot,
}

#[derive(Clone, Copy)]
pub struct Zone {
    pub bounds: Aabb,
    pub mode: ZoneMode,
}

impl Zone {
    pub const fn repeating(bounds: Aabb) -> Self {
        Self {
            bounds,
            mode: ZoneMode::Repeating,
        }
    }

    pub const fn one_shot(bounds: Aabb) -> Self {
        Self {
            bounds,
            mode: ZoneMode::OneShot,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneEvent {
    Entered(ZoneId),
    Stayed(ZoneId),
    Exited(ZoneId),
    /// Однократное действие: для `OneShot` только при первом входе.
    Triggered(ZoneId),
}

#[derive(Clone, Copy, Default)]
struct ZoneState {
    inside: bool,
    fired: bool,
}

pub struct ZoneSet {
    zones: Vec<Zone>,
    states: Vec<ZoneState>,
}

impl ZoneSet {
    pub fn new(zones: impl IntoIterator<Item = Zone>) -> Self {
        let zones: Vec<_> = zones.into_iter().collect();
        let states = vec![ZoneState::default(); zones.len()];
        Self { zones, states }
    }

    pub fn update(&mut self, player: Vec3) -> Vec<ZoneEvent> {
        self.update_inside(|bounds| bounds.contains(player))
    }

    pub fn update_bounds(&mut self, player: Aabb) -> Vec<ZoneEvent> {
        self.update_inside(|bounds| bounds.intersects(player))
    }

    fn update_inside(&mut self, is_inside: impl Fn(Aabb) -> bool) -> Vec<ZoneEvent> {
        let mut events = Vec::new();
        for (index, zone) in self.zones.iter().enumerate() {
            let id = ZoneId(index);
            let inside = is_inside(zone.bounds);
            let state = &mut self.states[index];
            let was_inside = state.inside;
            state.inside = inside;
            match zone.mode {
                ZoneMode::Repeating => {
                    if inside && !was_inside {
                        events.push(ZoneEvent::Entered(id));
                    } else if inside {
                        events.push(ZoneEvent::Stayed(id));
                    } else if was_inside {
                        events.push(ZoneEvent::Exited(id));
                    }
                }
                ZoneMode::OneShot => {
                    if inside && !was_inside {
                        events.push(ZoneEvent::Entered(id));
                        if !state.fired {
                            state.fired = true;
                            events.push(ZoneEvent::Triggered(id));
                        }
                    } else if inside {
                        events.push(ZoneEvent::Stayed(id));
                    } else if was_inside {
                        events.push(ZoneEvent::Exited(id));
                    }
                }
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use bfge::math::Vec3;

    use super::*;

    fn bounds(min: [f32; 3], max: [f32; 3]) -> Aabb {
        Aabb {
            min: Vec3::from_array(min),
            max: Vec3::from_array(max),
        }
    }

    fn at(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn repeating_zone_reports_enter_stay_and_exit() {
        let mut zones = ZoneSet::new([Zone::repeating(bounds([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))]);

        assert!(zones.update(at(-2.0, 1.0, 1.0)).is_empty());
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Stayed(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(-2.0, 1.0, 1.0)),
            vec![ZoneEvent::Exited(ZoneId(0))]
        );
    }

    #[test]
    fn repeating_zone_fires_again_after_exit() {
        let mut zones = ZoneSet::new([Zone::repeating(bounds([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))]);

        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(-2.0, 1.0, 1.0)),
            vec![ZoneEvent::Exited(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
    }

    #[test]
    fn one_shot_zone_triggers_once_but_tracks_presence() {
        let mut zones = ZoneSet::new([Zone::one_shot(bounds([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))]);

        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![
                ZoneEvent::Entered(ZoneId(0)),
                ZoneEvent::Triggered(ZoneId(0))
            ]
        );
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Stayed(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(-2.0, 1.0, 1.0)),
            vec![ZoneEvent::Exited(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(1.0, 1.0, 1.0)),
            vec![ZoneEvent::Stayed(ZoneId(0))]
        );
        assert_eq!(
            zones.update(at(-2.0, 1.0, 1.0)),
            vec![ZoneEvent::Exited(ZoneId(0))]
        );
    }

    #[test]
    fn zones_are_independent() {
        let mut zones = ZoneSet::new([
            Zone::repeating(bounds([0.0, 0.0, 0.0], [2.0, 2.0, 2.0])),
            Zone::repeating(bounds([1.0, 0.0, 0.0], [3.0, 2.0, 2.0])),
        ]);

        assert_eq!(
            zones.update(at(1.5, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0)), ZoneEvent::Entered(ZoneId(1))]
        );
        assert_eq!(
            zones.update(at(0.5, 1.0, 1.0)),
            vec![ZoneEvent::Stayed(ZoneId(0)), ZoneEvent::Exited(ZoneId(1))]
        );
    }

    #[test]
    fn bounds_update_detects_body_contact() {
        let mut zones = ZoneSet::new([Zone::repeating(bounds([-5.4, 0.0, 0.0], [-4.6, 0.2, 4.0]))]);
        let player = Aabb {
            min: Vec3::new(-5.0, 0.0, 2.0),
            max: Vec3::new(-4.5, 1.8, 2.5),
        };

        assert_eq!(
            zones.update_bounds(player),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
    }

    #[test]
    fn zone_boundary_is_inside() {
        let mut zones = ZoneSet::new([Zone::repeating(bounds([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))]);

        assert!(zones.update(at(-0.01, 1.0, 1.0)).is_empty());
        assert_eq!(
            zones.update(at(0.0, 1.0, 1.0)),
            vec![ZoneEvent::Entered(ZoneId(0))]
        );
    }
}
