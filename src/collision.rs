use bfge::math::Vec3;

use crate::world::Aabb;

// Постоянный объём охватывает фигуру при любом горизонтальном повороте.
pub const PLAYER_HALF_WIDTH: f32 = 0.5;
pub const PLAYER_HEIGHT: f32 = 1.75;
const CONTACT_GAP: f32 = 0.0001;
const TIME_EPSILON: f32 = 0.00001;
const HORIZONTAL_AXES: [usize; 2] = [0, 2];

pub fn player_bounds(feet: Vec3) -> Aabb {
    Aabb {
        min: feet - Vec3::new(PLAYER_HALF_WIDTH, 0.0, PLAYER_HALF_WIDTH),
        max: feet + Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HALF_WIDTH),
    }
}

/// Перемещает свободный от пересечений объём до контакта, затем вдоль стены.
/// Высота ног сохраняется; начальные пересечения должен исключить создатель сцены.
pub fn move_horizontal(mut feet: Vec3, displacement: Vec3, obstacles: &[Aabb]) -> Vec3 {
    let mut remaining = Vec3::new(displacement.x, 0.0, displacement.z);
    // Контакт может независимо заблокировать только две горизонтальные оси.
    for _ in 0..2 {
        let mut earliest = 1.0;
        let mut blocked = [false; 3];
        let player = player_bounds(feet);
        for obstacle in obstacles {
            if player.max.y <= obstacle.min.y || player.min.y >= obstacle.max.y {
                continue;
            }
            if let Some((time, axes)) = sweep(feet, remaining, *obstacle) {
                if time < earliest - TIME_EPSILON {
                    earliest = time;
                    blocked = axes;
                } else if (time - earliest).abs() <= TIME_EPSILON {
                    earliest = earliest.min(time);
                    for axis in HORIZONTAL_AXES {
                        blocked[axis] |= axes[axis];
                    }
                }
            }
        }
        feet += remaining * earliest;
        if !blocked[0] && !blocked[2] {
            return feet;
        }
        for axis in HORIZONTAL_AXES {
            if blocked[axis] {
                feet[axis] -= remaining[axis].signum() * CONTACT_GAP;
                remaining[axis] = 0.0;
            } else {
                remaining[axis] *= 1.0 - earliest;
            }
        }
    }
    feet
}

// Движущийся объём заменяется точкой в расширенном на полуширину препятствии.
fn sweep(origin: Vec3, delta: Vec3, obstacle: Aabb) -> Option<(f32, [bool; 3])> {
    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    let mut axes = [false; 3];
    for axis in HORIZONTAL_AXES {
        let min = obstacle.min[axis] - PLAYER_HALF_WIDTH;
        let max = obstacle.max[axis] + PLAYER_HALF_WIDTH;
        if delta[axis] == 0.0 {
            if origin[axis] <= min || origin[axis] >= max {
                return None;
            }
        } else {
            let first = (min - origin[axis]) / delta[axis];
            let second = (max - origin[axis]) / delta[axis];
            let near = first.min(second);
            if near > entry + TIME_EPSILON {
                axes = [false; 3];
                axes[axis] = true;
            } else if (near - entry).abs() <= TIME_EPSILON {
                axes[axis] = true;
            }
            entry = entry.max(near);
            exit = exit.min(first.max(second));
        }
    }
    if (0.0..=1.0).contains(&entry) && entry < exit {
        Some((entry, axes))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall() -> Aabb {
        Aabb {
            min: Vec3::new(2.0, 0.0, -10.0),
            max: Vec3::new(2.1, 2.0, 10.0),
        }
    }

    #[test]
    fn fast_motion_stops_before_a_thin_wall_from_both_sides() {
        let left = move_horizontal(Vec3::ZERO, Vec3::X * 100.0, &[wall()]);
        assert!((left.x - 1.5).abs() < 0.001);
        assert!(player_bounds(left).max.x <= wall().min.x);
        let right = move_horizontal(Vec3::new(5.0, 0.0, 0.0), -Vec3::X * 100.0, &[wall()]);
        assert!((right.x - 2.6).abs() < 0.001);
        assert!(player_bounds(right).min.x >= wall().max.x);
    }

    #[test]
    fn diagonal_motion_slides_along_wall() {
        let result = move_horizontal(Vec3::ZERO, Vec3::new(4.0, 0.0, 3.0), &[wall()]);
        assert!((result.x - 1.5).abs() < 0.001);
        assert!((result.z - 3.0).abs() < 0.001);
    }

    #[test]
    fn contact_allows_moving_away_and_along_wall() {
        let contact = Vec3::new(1.5, 0.0, 0.0);
        assert_eq!(
            move_horizontal(contact, -Vec3::X, &[wall()]),
            contact - Vec3::X
        );
        assert_eq!(
            move_horizontal(contact, Vec3::Z, &[wall()]),
            contact + Vec3::Z
        );
    }

    #[test]
    fn corner_remains_stable_under_repeated_pressure_in_either_order() {
        let perpendicular = Aabb {
            min: Vec3::new(-10.0, 0.0, 2.0),
            max: Vec3::new(10.0, 2.0, 2.1),
        };
        for obstacles in [[wall(), perpendicular], [perpendicular, wall()]] {
            let mut position = Vec3::ZERO;
            for _ in 0..120 {
                position = move_horizontal(position, Vec3::new(0.2, 0.0, 0.2), &obstacles);
            }
            let before = position;
            for _ in 0..120 {
                position = move_horizontal(position, Vec3::new(0.2, 0.0, 0.2), &obstacles);
                assert!(position.distance(before) < 0.00001);
            }
            assert!((position.x - 1.5).abs() < 0.001);
            assert!((position.z - 1.5).abs() < 0.001);
        }
    }

    #[test]
    fn diagonal_sweep_cannot_cross_a_finite_block() {
        let block = Aabb {
            min: Vec3::new(2.0, 0.0, 2.0),
            max: Vec3::new(3.0, 2.0, 3.0),
        };
        let result = move_horizontal(Vec3::ZERO, Vec3::new(5.0, 0.0, 5.0), &[block]);
        assert!(result.x < 1.5 && result.z < 1.5);
    }

    #[test]
    fn objects_below_feet_or_above_head_do_not_block_motion() {
        let ground = Aabb {
            min: Vec3::splat(-100.0),
            max: Vec3::new(100.0, 0.0, 100.0),
        };
        let overhead = Aabb {
            min: Vec3::new(0.0, PLAYER_HEIGHT, -10.0),
            max: Vec3::new(10.0, 3.0, 10.0),
        };
        let delta = Vec3::X * 4.0;
        assert_eq!(
            move_horizontal(Vec3::ZERO, delta, &[ground, overhead]),
            delta
        );
    }
}
