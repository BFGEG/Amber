use bfge::math::Vec3;

use crate::world::Aabb;

// Постоянный объём охватывает фигуру при любом горизонтальном повороте.
pub const PLAYER_HALF_WIDTH: f32 = 0.5;
pub const PLAYER_HEIGHT: f32 = 1.75;
// Опора считается по стопам, а не по всему телу: иначе персонаж висит над краем.
pub const FOOT_SUPPORT_HALF_WIDTH: f32 = 0.2;
const CONTACT_GAP: f32 = 0.0001;
const TIME_EPSILON: f32 = 0.00001;
const HORIZONTAL_AXES: [usize; 2] = [0, 2];

pub fn player_bounds(feet: Vec3) -> Aabb {
    Aabb {
        min: feet - Vec3::new(PLAYER_HALF_WIDTH, 0.0, PLAYER_HALF_WIDTH),
        max: feet + Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HALF_WIDTH),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerticalContact {
    Ground,
    Ceiling,
}

fn overlaps_horizontally(feet: Vec3, obstacle: Aabb, half_width: f32) -> bool {
    feet.x - half_width < obstacle.max.x
        && feet.x + half_width > obstacle.min.x
        && feet.z - half_width < obstacle.max.z
        && feet.z + half_width > obstacle.min.z
}

pub fn has_support(feet: Vec3, obstacles: &[Aabb]) -> bool {
    obstacles.iter().any(|obstacle| {
        overlaps_horizontally(feet, *obstacle, FOOT_SUPPORT_HALF_WIDTH)
            && (feet.y - obstacle.max.y).abs() <= CONTACT_GAP
    })
}

/// Проверяет весь вертикальный путь ног и головы, выбирая ближайшую поверхность.
pub fn move_vertical(
    mut feet: Vec3,
    displacement: f32,
    obstacles: &[Aabb],
) -> (Vec3, Option<VerticalContact>) {
    let mut target = feet.y + displacement;
    let mut contact = None;
    for obstacle in obstacles {
        if displacement < 0.0
            && overlaps_horizontally(feet, *obstacle, FOOT_SUPPORT_HALF_WIDTH)
            && feet.y >= obstacle.max.y
            && target <= obstacle.max.y
        {
            target = obstacle.max.y;
            contact = Some(VerticalContact::Ground);
        } else if displacement > 0.0
            && overlaps_horizontally(feet, *obstacle, PLAYER_HALF_WIDTH)
            && feet.y + PLAYER_HEIGHT <= obstacle.min.y
            && target + PLAYER_HEIGHT >= obstacle.min.y
        {
            target = obstacle.min.y - PLAYER_HEIGHT;
            contact = Some(VerticalContact::Ceiling);
        }
    }
    feet.y = target;
    (feet, contact)
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

    #[test]
    fn vertical_sweep_chooses_nearest_surface_in_either_order() {
        let lower = Aabb {
            min: Vec3::new(-2.0, -1.0, -2.0),
            max: Vec3::new(2.0, 0.0, 2.0),
        };
        let upper = Aabb {
            min: Vec3::new(-2.0, 2.0, -2.0),
            max: Vec3::new(2.0, 2.1, 2.0),
        };
        for obstacles in [[lower, upper], [upper, lower]] {
            let (position, contact) = move_vertical(Vec3::Y * 5.0, -100.0, &obstacles);
            assert_eq!(position.y, upper.max.y);
            assert_eq!(contact, Some(VerticalContact::Ground));
        }
    }

    #[test]
    fn exact_platform_edge_is_not_support() {
        let block = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        assert!(has_support(Vec3::new(0.5, 1.0, 0.5), &[block]));
        assert!(has_support(Vec3::new(1.1, 1.0, 0.5), &[block]));
        assert!(!has_support(Vec3::new(1.3, 1.0, 0.5), &[block]));
        assert!(!has_support(Vec3::new(1.5, 1.0, 0.5), &[block]));
    }

    #[test]
    fn landing_requires_foot_support_instead_of_body_overlap() {
        let block = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let (position, contact) = move_vertical(Vec3::new(1.3, 2.0, 0.5), -10.0, &[block]);
        assert_eq!(contact, None);
        assert!((position.y + 8.0).abs() < 0.00001);

        let (position, contact) = move_vertical(Vec3::new(1.1, 2.0, 0.5), -10.0, &[block]);
        assert_eq!(contact, Some(VerticalContact::Ground));
        assert_eq!(position.y, block.max.y);
    }

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
