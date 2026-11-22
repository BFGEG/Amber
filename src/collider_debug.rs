use bfge::math::Vec3;
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

use crate::world::Aabb;

/// Каркас из тонких экземпляров куба; использует общую геометрию сцены.
pub fn create_outline(
    scene: &mut dyn SceneApi,
    parent: SceneNodeId,
    cube: SceneObjectId,
    bounds: Aabb,
    color: [f32; 3],
) -> Result<Vec<SceneObjectId>, SceneError> {
    const THICKNESS: f32 = 0.025;
    let mut objects = Vec::new();
    let center = bounds.center();
    let half = bounds.size() * 0.5 + Vec3::splat(THICKNESS);
    for axis in 0..3 {
        let side = (axis + 1) % 3;
        let other_side = (axis + 2) % 3;
        for first in [-1.0, 1.0] {
            for second in [-1.0, 1.0] {
                let mut position = center;
                position[side] += first * half[side];
                position[other_side] += second * half[other_side];
                let mut size = Vec3::splat(THICKNESS);
                size[axis] = half[axis] * 2.0;
                let node = scene.create_node(parent)?;
                scene.set_node_transform(
                    node,
                    Transform {
                        translation: position,
                        scale: size,
                        ..Transform::default()
                    },
                )?;
                let object = scene.clone_object(cube, node)?;
                scene.set_object_color(object, color)?;
                scene.set_object_visible(object, false)?;
                objects.push(object);
            }
        }
    }
    Ok(objects)
}
