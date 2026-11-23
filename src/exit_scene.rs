use std::f32::consts::TAU;

use bfge::math::{self, Quat, Vec3};
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

use crate::world::{EXIT_CENTER, EXIT_RADIUS};

pub const PROMPT_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/exit-prompt.obj");

pub fn add_beacon(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
) -> Result<Vec<SceneObjectId>, SceneError> {
    const SEGMENTS: usize = 48;
    let mut objects = Vec::with_capacity(SEGMENTS * 2);
    let step = TAU / SEGMENTS as f32;
    let ring_segment = 2.0 * EXIT_RADIUS * (step * 0.5).tan();
    for i in 0..SEGMENTS {
        let angle = i as f32 * step;
        let base = EXIT_CENTER + EXIT_RADIUS * Vec3::new(angle.cos(), 0.0, angle.sin());
        let height = 18.0 + 10.0 * (i % 7) as f32 / 6.0;
        let color = if i % 3 == 0 {
            [0.95, 0.68, 0.28]
        } else {
            [1.0, 0.85, 0.55]
        };
        // Просветы между лучами оставляют видимым мир сквозь столп.
        let ray = Transform {
            translation: base + Vec3::Y * (height * 0.5),
            scale: Vec3::new(0.016, height, 0.016),
            ..Transform::default()
        };
        let rim = Transform {
            translation: base + Vec3::Y * 0.03,
            rotation: Quat::from_rotation_y(-angle),
            scale: Vec3::new(0.075, 0.028, ring_segment),
        };
        for (transform, color) in [(ray, color), (rim, [1.0, 0.72, 0.27])] {
            let node = scene.create_node(root)?;
            scene.set_node_transform(node, transform)?;
            let object = scene.clone_object(cube, node)?;
            scene.set_object_color(object, color)?;
            objects.push(object);
        }
    }
    Ok(objects)
}

pub struct ExitPrompt {
    node: SceneNodeId,
    card: SceneObjectId,
    visible: bool,
}

impl ExitPrompt {
    pub fn new(scene: &mut dyn SceneApi, card: SceneObjectId) -> Result<Self, SceneError> {
        let node = scene.create_node(scene.root_node())?;
        scene.reparent_object(card, node)?;
        scene.set_object_visible(card, false)?;
        Ok(Self {
            node,
            card,
            visible: false,
        })
    }

    pub fn synchronize(
        &mut self,
        scene: &mut dyn SceneApi,
        visible: bool,
    ) -> Result<(), SceneError> {
        if visible {
            let view = scene.viewpoint();
            let inverse_view = math::view(view.position, view.direction, view.roll).inverse();
            let rotation = Quat::from_mat4(&inverse_view);
            scene.set_node_transform(
                self.node,
                Transform {
                    translation: view.position + rotation * Vec3::new(0.0, -0.09, -0.25),
                    rotation,
                    ..Transform::default()
                },
            )?;
        }
        if self.visible != visible {
            scene.set_object_visible(self.card, visible)?;
            self.visible = visible;
        }
        Ok(())
    }
}
