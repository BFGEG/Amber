use bfge::ViewpointState;
use bfge::math::Vec3;

use crate::world::Aabb;

const CAMERA_DISTANCE: f32 = 10.0;
const CAMERA_CLEARANCE: f32 = 0.3;

pub fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(-70.0_f32.to_radians(), 15.0_f32.to_radians())
}

pub fn follow_player(
    viewpoint: &mut ViewpointState,
    feet: Vec3,
    direction: Vec3,
    obstacles: &[Aabb],
) {
    let target = feet + Vec3::Y;
    let backward = -direction.normalize();
    viewpoint.position =
        target + backward * clear_distance(target, backward, CAMERA_DISTANCE, obstacles);
    viewpoint.direction = direction;
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
        follow_player(&mut viewpoint, Vec3::ZERO, -Vec3::X, &obstacles);
        assert!(viewpoint.position.distance(Vec3::new(1.7, 1.0, 0.0)) < 0.00001);
        follow_player(&mut viewpoint, Vec3::Z * 2.0, -Vec3::X, &obstacles);
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
}
