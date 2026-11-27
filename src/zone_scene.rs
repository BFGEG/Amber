use bfge::math::{Quat, Vec3};
use bfge::{SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform};

use crate::collider_debug;
use crate::world::Aabb;
use crate::zone::{Zone, ZoneEvent, ZoneId, ZoneMode};

const REPEATING_FRAME_COLOR: [f32; 3] = [0.1, 0.8, 1.0];
const ONE_SHOT_FRAME_COLOR: [f32; 3] = [0.9, 0.3, 1.0];
const IDLE_COLOR: [f32; 3] = [1.2, 0.55, 0.08];
const ACTIVE_COLOR: [f32; 3] = [0.2, 1.4, 0.25];
const HAZARD_COLOR: [f32; 3] = [1.8, 0.08, 0.03];
const HAZARD_DARK_COLOR: [f32; 3] = [0.45, 0.015, 0.01];
const HAZARD_END_COLOR: [f32; 3] = [1.8, 1.2, 0.08];
const CHECKPOINT_ACTIVE_COLOR: [f32; 3] = [0.08, 0.65, 0.4];
const HAZARD_ALERT_COLOR: [f32; 3] = [2.0, 1.2, 0.15];
const MARKER_SIZE: f32 = 0.36;
const MARKER_HEIGHT: f32 = 1.1;
const HAZARD_SLAB_THICKNESS: f32 = 0.04;

struct ZoneMarker {
    id: ZoneId,
    object: SceneObjectId,
    active: bool,
}

pub struct ZoneVisuals {
    markers: Vec<ZoneMarker>,
}

struct HazardPart {
    object: SceneObjectId,
    color: [f32; 3],
}

pub struct HazardVisuals {
    hazards: Vec<Vec<HazardPart>>,
}

pub struct CheckpointVisuals {
    objects: Vec<SceneObjectId>,
    light: SceneObjectId,
    active: bool,
}

fn add_box(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
    center: Vec3,
    size: Vec3,
    color: [f32; 3],
) -> Result<SceneObjectId, SceneError> {
    let node = scene.create_node(root)?;
    scene.set_node_transform(
        node,
        Transform {
            translation: center,
            scale: size,
            ..Transform::default()
        },
    )?;
    let object = scene.clone_object(cube, node)?;
    scene.set_object_color(object, color)?;
    Ok(object)
}

pub fn add_checkpoint_visuals(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
    light: SceneObjectId,
    bounds: Aabb,
) -> Result<CheckpointVisuals, SceneError> {
    let center = Vec3::new(bounds.center().x, bounds.min.y + 0.015, bounds.center().z);
    let node = scene.create_node(root)?;
    scene.set_node_transform(node, Transform::from_translation(center))?;
    scene.reparent_object(light, node)?;
    scene.set_object_color(light, IDLE_COLOR)?;
    let mut objects = Vec::new();
    const SEGMENTS: usize = 48;
    const RADIUS: f32 = 1.05;
    for index in 0..SEGMENTS {
        let angle = std::f32::consts::TAU * index as f32 / SEGMENTS as f32;
        let node = scene.create_node(root)?;
        scene.set_node_transform(
            node,
            Transform {
                translation: center + Vec3::new(RADIUS * angle.cos(), 0.0, RADIUS * angle.sin()),
                rotation: Quat::from_rotation_y(-angle),
                scale: Vec3::new(
                    0.065,
                    0.025,
                    std::f32::consts::TAU * RADIUS / SEGMENTS as f32 * 1.03,
                ),
            },
        )?;
        let object = scene.clone_object(cube, node)?;
        scene.set_object_color(object, IDLE_COLOR)?;
        objects.push(object);
    }
    Ok(CheckpointVisuals {
        objects,
        light,
        active: false,
    })
}

impl CheckpointVisuals {
    pub fn activate(&mut self, scene: &mut dyn SceneApi) -> Result<(), SceneError> {
        if !self.active {
            for object in &self.objects {
                scene.set_object_color(*object, CHECKPOINT_ACTIVE_COLOR)?;
            }
            scene.set_object_visible(self.light, false)?;
            self.active = true;
        }
        Ok(())
    }
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
            object,
            active: false,
        });
    }
    Ok(ZoneVisuals { markers })
}

pub fn add_hazard_visuals(
    scene: &mut dyn SceneApi,
    root: SceneNodeId,
    cube: SceneObjectId,
    hazards: &[Zone],
) -> Result<HazardVisuals, SceneError> {
    let mut visual_hazards = Vec::with_capacity(hazards.len());
    for hazard in hazards {
        let mut objects = Vec::new();
        for object in
            collider_debug::create_outline(scene, root, cube, hazard.bounds, HAZARD_COLOR)?
        {
            scene.set_object_visible(object, true)?;
            objects.push(HazardPart {
                object,
                color: HAZARD_COLOR,
            });
        }
        let center = hazard.bounds.center();
        let size = hazard.bounds.size();
        let y = hazard.bounds.max.y - HAZARD_SLAB_THICKNESS * 0.5;
        // Объёмная ловушка под платформами сохраняет сплошную красную плиту.
        if size.y > HAZARD_SLAB_THICKNESS * 2.0 {
            let object = add_box(
                scene,
                root,
                cube,
                Vec3::new(center.x, y, center.z),
                Vec3::new(size.x, HAZARD_SLAB_THICKNESS, size.z),
                HAZARD_COLOR,
            )?;
            objects.push(HazardPart {
                object,
                color: HAZARD_COLOR,
            });
        } else {
            const SEGMENTS: usize = 8;
            const END_WIDTH: f32 = 0.12;
            let segment_width = (size.z - END_WIDTH * 2.0) / SEGMENTS as f32;
            for index in 0..SEGMENTS {
                let color = if index % 2 == 0 {
                    HAZARD_COLOR
                } else {
                    HAZARD_DARK_COLOR
                };
                let z = hazard.bounds.min.z + END_WIDTH + (index as f32 + 0.5) * segment_width;
                let object = add_box(
                    scene,
                    root,
                    cube,
                    Vec3::new(center.x, y, z),
                    Vec3::new(size.x, HAZARD_SLAB_THICKNESS, segment_width),
                    color,
                )?;
                objects.push(HazardPart { object, color });
            }
            for z in [
                hazard.bounds.min.z + END_WIDTH * 0.5,
                hazard.bounds.max.z - END_WIDTH * 0.5,
            ] {
                let object = add_box(
                    scene,
                    root,
                    cube,
                    Vec3::new(center.x, y, z),
                    Vec3::new(size.x, HAZARD_SLAB_THICKNESS, END_WIDTH),
                    HAZARD_END_COLOR,
                )?;
                objects.push(HazardPart {
                    object,
                    color: HAZARD_END_COLOR,
                });
            }
        }
        visual_hazards.push(objects);
    }
    Ok(HazardVisuals {
        hazards: visual_hazards,
    })
}

impl HazardVisuals {
    pub fn set_alert(
        &self,
        scene: &mut dyn SceneApi,
        id: ZoneId,
        alert: bool,
    ) -> Result<(), SceneError> {
        let Some(objects) = self.hazards.get(id.0) else {
            return Ok(());
        };
        for part in objects {
            scene.set_object_color(
                part.object,
                if alert {
                    HAZARD_ALERT_COLOR
                } else {
                    part.color
                },
            )?;
        }
        Ok(())
    }
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
                ZoneEvent::Stayed(_) | ZoneEvent::Triggered(_) => continue,
                ZoneEvent::Exited(id) => (id, false),
            };
            let Some(marker) = self.markers.iter_mut().find(|marker| marker.id == id) else {
                continue;
            };
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
