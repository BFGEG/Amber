use bfge::{SceneApi, SceneError, SceneObjectId};

use crate::{
    scene_state::{SceneKind, SceneState},
    section_scene::SectionVisuals,
    zone_scene::{CheckpointVisuals, HazardVisuals, ZoneVisuals},
};

/// Два заранее построенных набора геометрии. Загрузка меняет видимость и состояние,
/// не создавая новых узлов, персонажей и обработчиков в цикле кадров.
pub struct SceneVisuals {
    pub primary_objects: Vec<SceneObjectId>,
    pub demo_objects: Vec<SceneObjectId>,
    pub primary_outlines: Vec<SceneObjectId>,
    pub demo_outlines: Vec<SceneObjectId>,
    pub player_outlines: Vec<SceneObjectId>,
    pub zones: ZoneVisuals,
    pub hazards: HazardVisuals,
    pub checkpoint: Option<CheckpointVisuals>,
    pub section: Option<SectionVisuals>,
}

fn visibility(
    scene: &mut dyn SceneApi,
    objects: &[SceneObjectId],
    visible: bool,
) -> Result<(), SceneError> {
    for object in objects {
        scene.set_object_visible(*object, visible)?;
    }
    Ok(())
}

impl SceneVisuals {
    pub fn activate(
        &mut self,
        scene: &mut dyn SceneApi,
        state: &SceneState,
        debug: bool,
    ) -> Result<(), SceneError> {
        let primary = state.kind != SceneKind::Demo;
        visibility(scene, &self.primary_objects, primary)?;
        visibility(scene, &self.demo_objects, !primary)?;
        self.zones.set_visible(scene, primary)?;
        self.zones.synchronize(scene, &state.zone_set)?;
        self.hazards.reset(scene)?;
        self.hazards.set_visible(scene, primary)?;
        if let Some(checkpoint) = &mut self.checkpoint {
            checkpoint.reset(scene)?;
            checkpoint.set_visible(scene, primary)?;
        }
        if let Some(section) = &mut self.section {
            section.synchronize(scene, &state.section)?;
            section.set_visible(scene, primary)?;
        }
        self.set_debug(scene, state.kind, debug)
    }

    pub fn set_debug(
        &self,
        scene: &mut dyn SceneApi,
        kind: SceneKind,
        enabled: bool,
    ) -> Result<(), SceneError> {
        visibility(
            scene,
            &self.primary_outlines,
            enabled && kind != SceneKind::Demo,
        )?;
        visibility(
            scene,
            &self.demo_outlines,
            enabled && kind == SceneKind::Demo,
        )?;
        visibility(scene, &self.player_outlines, enabled)
    }
}
