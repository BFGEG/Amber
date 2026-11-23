use bfge::math::{self, Quat};
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

pub const CARD_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/pause-card.obj");

/// Карточка с заранее отрисованным текстом, закреплённая перед камерой.
/// Модель загружается один раз; пауза меняет только transform и видимость.
pub struct PauseOverlay {
    root: SceneNodeId,
    card: SceneObjectId,
    visible: bool,
}

impl PauseOverlay {
    pub fn new(scene: &mut dyn SceneApi, card: SceneObjectId) -> Result<Self, SceneError> {
        let root = scene.create_node(scene.root_node())?;
        scene.reparent_object(card, root)?;
        scene.set_object_visible(card, false)?;
        Ok(Self {
            root,
            card,
            visible: false,
        })
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
            scene.set_object_visible(self.card, paused)?;
            self.visible = paused;
        }
        Ok(())
    }
}
