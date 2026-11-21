use bfge::ViewpointState;
use bfge::math::Vec3;

use crate::world::Aabb;

const CAMERA_DISTANCE: f32 = 10.0;
const CAMERA_CLEARANCE: f32 = 0.3;
const HEIGHT_RESPONSE: f32 = 6.0;
const MAX_HEIGHT_OFFSET: f32 = 0.65;
const MAX_FRAME_DELTA: f32 = 0.1;

pub fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(-70.0_f32.to_radians(), 15.0_f32.to_radians())
}

pub struct FollowCamera {
    height: f32,
}

impl FollowCamera {
    pub fn new(feet: Vec3) -> Self {
        Self { height: feet.y }
    }

    pub fn update_height(&mut self, feet: Vec3, dt: f32) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, MAX_FRAME_DELTA)
        } else {
            0.0
        };
        let blend = 1.0 - (-HEIGHT_RESPONSE * dt).exp();
        self.height += (feet.y - self.height) * blend;
        // При быстром падении персонаж остаётся рядом с центром кадра.
        self.height = self
            .height
            .clamp(feet.y - MAX_HEIGHT_OFFSET, feet.y + MAX_HEIGHT_OFFSET);
    }

    pub fn place(
        &self,
        viewpoint: &mut ViewpointState,
        feet: Vec3,
        direction: Vec3,
        obstacles: &[Aabb],
    ) {
        let target = feet + Vec3::Y;
        let offset = Vec3::Y * (self.height - feet.y) - direction.normalize() * CAMERA_DISTANCE;
        let backward = offset.normalize();
        let distance = clear_distance(target, backward, offset.length(), obstacles);
        viewpoint.position = target + backward * distance;
        viewpoint.direction = -backward;
    }
}

fn clear_distance(origin: Vec3, direction: Vec3, distance: f32, obstacles: &[Aabb]) -> f32 {
    obstacles.iter().fold(distance, |limit, obstacle| {
        obstacle_entry(origin, direction, limit, *obstacle).unwrap_or(limit)
    })
}

fn obstacle_entry(origin: Vec3, direction: Vec3, limit: f32, obstacle: Aabb) -> Option<f32> {
    // Запас вокруг грани оставляет ближнюю плоскость камеры перед стеной.
    let min = obstacle.min - Vec3::splat(CAMERA_CLEARANCE);
    let max = obstacle.max + Vec3::splat(CAMERA_CLEARANCE);
    let mut entry: f32 = 0.0;
    let mut exit = limit;
    for axis in 0..3 {
        if direction[axis].abs() < 1e-6 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let first = (min[axis] - origin[axis]) / direction[axis];
        let second = (max[axis] - origin[axis]) / direction[axis];
        entry = entry.max(first.min(second));
        exit = exit.min(first.max(second));
        if entry > exit {
            return None;
        }
    }
    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_stops_before_the_nearest_wall_and_returns_when_clear() {
        let wall = |x: f32| Aabb {
            min: Vec3::new(x, 0.0, -1.0),
            max: Vec3::new(x + 0.5, 3.0, 1.0),
        };
        let obstacles = [wall(5.0), wall(2.0)];
        let mut viewpoint = ViewpointState {
            position: Vec3::ZERO,
            direction: Vec3::X,
            roll: 0.0,
            fov_y: 55.0_f32.to_radians(),
        };
        let camera = FollowCamera::new(Vec3::ZERO);
        camera.place(&mut viewpoint, Vec3::ZERO, -Vec3::X, &obstacles);
        assert!(viewpoint.position.distance(Vec3::new(1.7, 1.0, 0.0)) < 0.00001);
        camera.place(&mut viewpoint, Vec3::Z * 2.0, -Vec3::X, &obstacles);
        assert_eq!(viewpoint.position, Vec3::new(10.0, 1.0, 2.0));
        assert_eq!(viewpoint.direction, -Vec3::X);
        assert_eq!(viewpoint.fov_y, 55.0_f32.to_radians());
    }

    #[test]
    fn camera_arm_checks_vertical_obstacles_and_ignores_walls_behind_the_target() {
        let ground = crate::world::GROUND.bounds();
        let downward = Vec3::new(1.0, -1.0, 0.0).normalize();
        let target = Vec3::Y;
        let distance = clear_distance(target, downward, CAMERA_DISTANCE, &[ground]);
        assert!((target.y + downward.y * distance - CAMERA_CLEARANCE).abs() < 0.00001);
        assert_eq!(
            clear_distance(target, -downward, CAMERA_DISTANCE, &[ground]),
            CAMERA_DISTANCE,
        );
    }

    #[test]
    fn follows_jumps_and_landings_at_the_same_rate_at_different_fps() {
        let elevated = Vec3::Y * 0.5;
        let mut slower = FollowCamera::new(Vec3::ZERO);
        let mut faster = FollowCamera::new(Vec3::ZERO);
        slower.update_height(elevated, 1.0 / 30.0);
        faster.update_height(elevated, 1.0 / 60.0);
        faster.update_height(elevated, 1.0 / 60.0);
        assert!(slower.height > 0.0 && slower.height < elevated.y);
        assert!((slower.height - faster.height).abs() < 0.00001);

        for _ in 0..120 {
            slower.update_height(elevated, 1.0 / 60.0);
        }
        assert!((slower.height - elevated.y).abs() < 0.00001);
        slower.update_height(Vec3::Y * -8.0, 1.0 / 60.0);
        assert!((slower.height + 8.0 - MAX_HEIGHT_OFFSET).abs() < 0.00001);
    }

    #[test]
    fn respawn_places_the_camera_at_the_new_target_without_using_the_old_height() {
        let start = crate::world::START_POSITION;
        let direction = Vec3::new(1.0, -0.2, 0.0).normalize();
        let mut viewpoint = ViewpointState {
            position: Vec3::ZERO,
            direction,
            roll: 0.0,
            fov_y: 55.0_f32.to_radians(),
        };
        let previous = Vec3::new(500.0, -20.0, 70.0);
        FollowCamera::new(previous).place(&mut viewpoint, previous, direction, &[]);
        FollowCamera::new(start).place(&mut viewpoint, start, direction, &[]);
        let target = viewpoint.position + viewpoint.direction * CAMERA_DISTANCE;
        assert!(target.distance(start + Vec3::Y) < 0.00001);
        assert!(viewpoint.direction.distance(direction) < 0.00001);
    }
}
