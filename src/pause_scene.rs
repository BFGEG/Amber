use bfge::math::{self, Quat, Vec3};
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

/// Небольшая надпись из общей геометрии куба, закреплённая перед камерой.
pub struct PauseOverlay {
    root: SceneNodeId,
    objects: Vec<SceneObjectId>,
    visible: bool,
}

impl PauseOverlay {
    pub fn new(scene: &mut dyn SceneApi, cube: SceneObjectId) -> Result<Self, SceneError> {
        let root = scene.create_node(scene.root_node())?;
        let mut overlay = Self {
            root,
            objects: Vec::new(),
            visible: false,
        };
        overlay.add_box(
            scene,
            cube,
            Vec3::ZERO,
            Vec3::new(0.24, 0.16, 0.004),
            [0.02, 0.03, 0.04],
        )?;
        overlay.add_text(scene, cube, "ПАУЗА", 0.006, 0.065)?;
        overlay.add_text(scene, cube, "ESC", 0.005, -0.025)?;
        Ok(overlay)
    }

    fn add_box(
        &mut self,
        scene: &mut dyn SceneApi,
        cube: SceneObjectId,
        position: Vec3,
        size: Vec3,
        color: [f32; 3],
    ) -> Result<(), SceneError> {
        let node = scene.create_node(self.root)?;
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
        self.objects.push(object);
        Ok(())
    }

    fn add_text(
        &mut self,
        scene: &mut dyn SceneApi,
        cube: SceneObjectId,
        text: &str,
        pixel: f32,
        top: f32,
    ) -> Result<(), SceneError> {
        let width = (text.chars().count() * 6 - 1) as f32 * pixel;
        for (column, character) in text.chars().enumerate() {
            for (row, bits) in glyph(character).into_iter().enumerate() {
                for x in 0..5 {
                    if bits & (1 << (4 - x)) != 0 {
                        self.add_box(
                            scene,
                            cube,
                            Vec3::new(
                                -width * 0.5 + (column * 6 + x) as f32 * pixel + pixel * 0.5,
                                top - row as f32 * pixel,
                                0.004,
                            ),
                            Vec3::new(pixel * 0.92, pixel * 0.92, 0.002),
                            [2.0; 3],
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn synchronize(
        &mut self,
        scene: &mut dyn SceneApi,
        paused: bool,
    ) -> Result<(), SceneError> {
        if self.visible != paused {
            if paused {
                let view = scene.viewpoint();
                let inverse_view = math::view(view.position, view.direction, view.roll).inverse();
                scene.set_node_transform(
                    self.root,
                    Transform {
                        translation: view.position + view.direction.normalize() * 0.25,
                        rotation: Quat::from_mat4(&inverse_view),
                        ..Transform::default()
                    },
                )?;
            }
            for object in &self.objects {
                scene.set_object_visible(*object, paused)?;
            }
            self.visible = paused;
        }
        Ok(())
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'П' => [31, 17, 17, 17, 17, 17, 17],
        'А' => [14, 17, 17, 31, 17, 17, 17],
        'У' => [17, 17, 10, 4, 8, 16, 16],
        'З' => [30, 1, 1, 14, 1, 1, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        _ => [0; 7],
    }
}
