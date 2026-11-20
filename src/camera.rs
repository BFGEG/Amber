use bfge::ViewpointState;
use bfge::math::Vec3;

const CAMERA_DISTANCE: f32 = 10.0;

/// Следует за текущим положением сразу, в том числе после возврата на старт.
pub fn follow_player(viewpoint: &mut ViewpointState, feet: Vec3, direction: Vec3) {
    viewpoint.position = feet + Vec3::Y - direction.normalize() * CAMERA_DISTANCE;
    viewpoint.direction = direction;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respawn_repositions_camera_without_changing_view_parameters() {
        let mut viewpoint = ViewpointState {
            position: Vec3::ZERO,
            direction: Vec3::X,
            roll: 0.0,
            fov_y: 55.0_f32.to_radians(),
        };
        let direction = Vec3::new(1.0, -0.2, 0.0).normalize();
        follow_player(&mut viewpoint, Vec3::new(500.0, -20.0, 70.0), direction);
        follow_player(&mut viewpoint, crate::world::START_POSITION, direction);
        let target = viewpoint.position + direction * CAMERA_DISTANCE;
        assert!(target.distance(crate::world::START_POSITION + Vec3::Y) < 0.00001);
        assert_eq!(viewpoint.direction, direction);
        assert_eq!(viewpoint.fov_y, 55.0_f32.to_radians());
    }
}
