mod simulation;

use std::error::Error;
use std::path::Path;

use bfge::math::{Quat, Vec3};
use bfge::{
    DirectionalLight, Engine, EngineApi, EngineEvent, KeyCode, SceneLighting, Transform,
    ViewpointState, WindowDescriptor,
};

const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cube.obj");
const GROUND_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ground.obj");
const INITIAL_DIRECTION: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
const MOUSE_SENSITIVITY: f32 = 0.002;
const CAMERA_DISTANCE: f32 = 10.0;
const INITIAL_POSITION: Vec3 = Vec3::new(-4.0, 0.0, 2.0);

fn build_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let ground = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        ground,
        Transform {
            translation: Vec3::new(40.0, 0.0, 0.0),
            scale: Vec3::new(160.0, 1.0, 200.0),
            ..Transform::default()
        },
    )?;
    engine.load_model(Path::new(GROUND_MODEL), ground)?;

    let corwin = engine.scene().create_node(root)?;
    let mut player = simulation::Player::new(INITIAL_POSITION);
    engine.scene().set_node_transform(
        corwin,
        Transform {
            translation: player.position,
            rotation: Quat::IDENTITY,
            ..Transform::default()
        },
    )?;
    let torso = engine.scene().create_node(corwin)?;
    engine.scene().set_node_transform(
        torso,
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            scale: Vec3::new(0.35, 0.7, 0.55),
            ..Transform::default()
        },
    )?;
    let cube = engine.load_model(Path::new(CUBE_MODEL), torso)?;
    engine.scene().set_object_color(cube, [0.35, 0.05, 0.05])?;

    let body_parts = [
        ([0.0, 1.55, 0.0], [0.35, 0.4, 0.35], [0.75, 0.45, 0.3]),
        ([0.0, 1.0, -0.37], [0.26, 0.65, 0.19], [0.18, 0.025, 0.025]),
        ([0.0, 1.0, 0.37], [0.26, 0.65, 0.19], [0.18, 0.025, 0.025]),
        ([0.0, 0.325, -0.16], [0.3, 0.65, 0.22], [0.07, 0.08, 0.1]),
        ([0.0, 0.325, 0.16], [0.3, 0.65, 0.22], [0.07, 0.08, 0.1]),
    ];
    for (position, size, color) in body_parts {
        let node = engine.scene().create_node(corwin)?;
        engine.scene().set_node_transform(
            node,
            Transform {
                translation: Vec3::from_array(position),
                scale: Vec3::from_array(size),
                ..Transform::default()
            },
        )?;
        let object = engine.scene().clone_object(cube, node)?;
        engine.scene().set_object_color(object, color)?;
    }

    let sky = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        sky,
        Transform {
            translation: Vec3::new(70.0, 50.0, 2.0),
            scale: Vec3::new(0.1, 100.0, 160.0),
            ..Transform::default()
        },
    )?;
    let backdrop = engine.scene().clone_object(cube, sky)?;
    engine.scene().set_object_color(backdrop, [0.7, 1.4, 2.4])?;

    let blocks = [
        ([-0.5, 0.25, -1.0], [1.4, 0.5, 1.4]),
        ([1.5, 0.5, 2.5], [1.4, 1.0, 1.4]),
        ([4.0, 0.75, -1.5], [1.4, 1.5, 1.4]),
        ([6.0, 1.0, 2.5], [1.4, 2.0, 1.4]),
        ([8.0, 1.25, -1.5], [1.4, 2.5, 1.4]),
    ];
    for (position, size) in blocks {
        let node = engine.scene().create_node(root)?;
        engine.scene().set_node_transform(
            node,
            Transform {
                translation: Vec3::from_array(position),
                scale: Vec3::from_array(size),
                ..Transform::default()
            },
        )?;
        let object = engine.scene().clone_object(cube, node)?;
        engine
            .scene()
            .set_object_color(object, [0.25, 0.25, 0.25])?;
    }

    engine.scene().set_viewpoint(ViewpointState {
        position: Vec3::new(-10.0, 3.1, 2.0),
        direction: Vec3::new(1.0, -0.22, 0.0),
        roll: 0.0,
        fov_y: 55.0_f32.to_radians(),
    })?;
    engine.scene().set_lighting(SceneLighting {
        ambient: Vec3::splat(0.3),
        directional: DirectionalLight {
            direction: Vec3::new(-0.5, -1.0, -0.7),
            color: Vec3::splat(0.8),
        },
    })?;
    let initial_direction = INITIAL_DIRECTION.normalize();

    let mut yaw = initial_direction.x.atan2(-initial_direction.z);
    let mut pitch = initial_direction.y.asin();
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let input = context.input();

            let (dx, dy) = input.mouse_delta();
            pitch -= dy as f32 * MOUSE_SENSITIVITY;
            yaw += dx as f32 * MOUSE_SENSITIVITY;

            let (yaw_sin, yaw_cos) = yaw.sin_cos();
            let (pitch_sin, pitch_cos) = pitch.sin_cos();

            let forward = Vec3::new(yaw_sin, 0.0, -yaw_cos);
            let right = Vec3::new(yaw_cos, 0.0, yaw_sin);

            let mut movement = Vec3::ZERO;
            if input.is_key_pressed(KeyCode::KeyW) {
                movement += forward;
            }
            if input.is_key_pressed(KeyCode::KeyS) {
                movement -= forward;
            }
            if input.is_key_pressed(KeyCode::KeyD) {
                movement += right;
            }
            if input.is_key_pressed(KeyCode::KeyA) {
                movement -= right;
            }

            player.update(movement, time.time_since_last_frame);
            context.scene().set_node_transform(
                corwin,
                Transform {
                    translation: player.position,
                    rotation: player.rotation,
                    ..Transform::default()
                },
            )?;

            let mut viewpoint = context.scene().viewpoint();
            let direction = Vec3::new(yaw_sin * pitch_cos, pitch_sin, -yaw_cos * pitch_cos);
            viewpoint.position = player.position - direction.normalize() * CAMERA_DISTANCE
                + Vec3::new(0.0, 1.0, 0.0);
            viewpoint.direction = direction;
            context.scene().set_viewpoint(viewpoint)?;
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new("Amber — тестовая сцена", 1280, 720))?;
    let result = build_scene(&mut engine).and_then(|()| engine.run().map_err(Into::into));
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}
