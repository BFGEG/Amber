use bfge::math::Vec3;
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

use crate::{collider_debug, section::SectionState, world::Aabb};

const INITIAL_COLOR: [f32; 3] = [1.2, 0.55, 0.08];
const CHANGED_COLOR: [f32; 3] = [0.1, 0.7, 1.4];

pub struct SectionVisuals {
    node: SceneNodeId,
    object: SceneObjectId,
    initial_position: Vec3,
    changed: bool,
    outlines: Vec<SceneObjectId>,
}

pub fn add_to_scene(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
    bounds: Aabb,
) -> Result<SectionVisuals, SceneError> {
    let outlines = collider_debug::create_outline(scene, root, cube, bounds, CHANGED_COLOR)?;
    for object in &outlines {
        scene.set_object_visible(*object, true)?;
    }
    let center = bounds.center();
    let initial_position = Vec3::new(center.x, bounds.min.y + 1.0, center.z);
    let node = scene.create_node(root)?;
    scene.set_node_transform(
        node,
        Transform {
            translation: initial_position,
            scale: Vec3::splat(0.6),
            ..Transform::default()
        },
    )?;
    let object = scene.clone_object(cube, node)?;
    scene.set_object_color(object, INITIAL_COLOR)?;
    Ok(SectionVisuals {
        node,
        object,
        initial_position,
        changed: false,
        outlines,
    })
}

impl SectionVisuals {
    pub fn set_visible(&self, scene: &mut dyn SceneApi, visible: bool) -> Result<(), SceneError> {
        scene.set_object_visible(self.object, visible)?;
        for object in &self.outlines {
            scene.set_object_visible(*object, visible)?;
        }
        Ok(())
    }

    pub fn synchronize(
        &mut self,
        scene: &mut dyn SceneApi,
        state: &SectionState,
    ) -> Result<(), SceneError> {
        if self.changed != state.changed {
            let offset = if state.changed {
                Vec3::Y * 0.75
            } else {
                Vec3::ZERO
            };
            scene.set_node_transform(
                self.node,
                Transform {
                    translation: self.initial_position + offset,
                    scale: Vec3::splat(0.6),
                    ..Transform::default()
                },
            )?;
            scene.set_object_color(
                self.object,
                if state.changed {
                    CHANGED_COLOR
                } else {
                    INITIAL_COLOR
                },
            )?;
            self.changed = state.changed;
        }
        Ok(())
    }
}
