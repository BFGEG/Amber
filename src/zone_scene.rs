use bfge::math::Vec3;
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

use crate::collider_debug;
use crate::zone::{Zone, ZoneEvent, ZoneId, ZoneMode};

const REPEATING_FRAME_COLOR: [f32; 3] = [0.1, 0.8, 1.0];
const ONE_SHOT_FRAME_COLOR: [f32; 3] = [0.9, 0.3, 1.0];
const IDLE_COLOR: [f32; 3] = [1.2, 0.55, 0.08];
const ACTIVE_COLOR: [f32; 3] = [0.2, 1.4, 0.25];
const MARKER_SIZE: f32 = 0.36;
const MARKER_HEIGHT: f32 = 1.1;

struct ZoneMarker {
    id: ZoneId,
    mode: ZoneMode,
    object: SceneObjectId,
    active: bool,
}

pub struct ZoneVisuals {
    markers: Vec<ZoneMarker>,
}

pub fn add_to_scene(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
    zones: &[Zone],
) -> Result<ZoneVisuals, SceneError> {
    let mut markers = Vec::with_capacity(zones.len());
    for (index, zone) in zones.iter().enumerate() {
        let frame_color = match zone.mode {
            ZoneMode::Repeating => REPEATING_FRAME_COLOR,
            ZoneMode::OneShot => ONE_SHOT_FRAME_COLOR,
        };
        for object in collider_debug::create_outline(scene, root, cube, zone.bounds, frame_color)? {
            scene.set_object_visible(object, true)?;
        }

        let center = zone.bounds.center();
        let node = scene.create_node(root)?;
        scene.set_node_transform(
            node,
            Transform {
                translation: Vec3::new(center.x, zone.bounds.min.y + MARKER_HEIGHT, center.z),
                scale: Vec3::splat(MARKER_SIZE),
                ..Transform::default()
            },
        )?;
        let object = scene.clone_object(cube, node)?;
        scene.set_object_color(object, IDLE_COLOR)?;
        markers.push(ZoneMarker {
            id: ZoneId(index),
            mode: zone.mode,
            object,
            active: false,
        });
    }
    Ok(ZoneVisuals { markers })
}

impl ZoneVisuals {
    pub fn apply(
        &mut self,
        scene: &mut dyn SceneApi,
        events: &[ZoneEvent],
    ) -> Result<(), SceneError> {
        for event in events {
            let (id, active) = match *event {
                ZoneEvent::Entered(id) => (id, true),
                ZoneEvent::Stayed(_) => continue,
                ZoneEvent::Exited(id) => (id, false),
            };
            let Some(marker) = self.markers.iter_mut().find(|marker| marker.id == id) else {
                continue;
            };
            if marker.mode == ZoneMode::OneShot && !active {
                continue;
            }
            if marker.active == active {
                continue;
            }
            marker.active = active;
            scene.set_object_color(
                marker.object,
                if active { ACTIVE_COLOR } else { IDLE_COLOR },
            )?;
        }
        Ok(())
    }
}
