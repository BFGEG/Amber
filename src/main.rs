mod collider_debug;
mod collision;
mod devtools;
mod input;
mod simulation;
mod world;

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

fn build_scene(
    engine: &mut Engine,
    timing_demo: bool,
    mut debug_visible: bool,
    mut limiter: devtools::FrameLimiter,
) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let ground = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        ground,
        Transform {
            translation: Vec3::new(world::GROUND.center.x, 0.0, world::GROUND.center.z),
            scale: Vec3::new(world::GROUND.size.x, 1.0, world::GROUND.size.z),
            ..Transform::default()
        },
    )?;
    engine.load_model(Path::new(GROUND_MODEL), ground)?;

    let corwin = engine.scene().create_node(root)?;
    // Для сравнения FPS оставляем свободную полосу в стороне от препятствий.
    let start_position = if timing_demo {
        Vec3::new(-4.0, 0.0, -6.0)
    } else {
        INITIAL_POSITION
    };
    let mut player = simulation::Player::new(start_position);
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

    for block in world::PLATFORMS
        .into_iter()
        .chain(world::WALLS)
        .chain(world::VERTICAL_COURSE)
    {
        let node = engine.scene().create_node(root)?;
        engine.scene().set_node_transform(
            node,
            Transform {
                translation: block.center,
                scale: block.size,
                ..Transform::default()
            },
        )?;
        let object = engine.scene().clone_object(cube, node)?;
        engine
            .scene()
            .set_object_color(object, [0.25, 0.25, 0.25])?;
    }

    let colliders = world::static_colliders();
    let mut outlines = Vec::new();
    for bounds in &colliders {
        outlines.extend(collider_debug::create_outline(
            engine.scene(),
            root,
            cube,
            *bounds,
            [0.1, 1.0, 0.2],
        )?);
    }
    let player_outline = engine.scene().create_node(root)?;
    outlines.extend(collider_debug::create_outline(
        engine.scene(),
        player_outline,
        cube,
        collision::player_bounds(Vec3::ZERO),
        [1.0, 0.65, 0.05],
    )?);
    engine
        .scene()
        .set_node_transform(player_outline, Transform::from_translation(player.position))?;
    for object in &outlines {
        engine.scene().set_object_visible(*object, debug_visible)?;
    }
    let mut debug_pressed = false;

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
    let initial_direction = if timing_demo {
        // Прогон направлен вдоль платформ, как исходный обзор сцены.
        -INITIAL_DIRECTION
    } else {
        INITIAL_DIRECTION
    }
    .normalize();

    let mut yaw = initial_direction.x.atan2(-initial_direction.z);
    let mut pitch = initial_direction.y.asin();
    let mut clock = simulation::FixedClock::default();
    let mut jump_input = input::JumpInput::default();
    let mut demo_steps = 0;
    let mut demo_frames = 0;
    let mut demo_elapsed = 0.0;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let input = context.input();
            let Some(steps) = clock.frame_steps(time.time_since_last_frame, input.is_focused()) else {
                jump_input.pause(input.is_key_pressed(KeyCode::Space));
                debug_pressed = false;
                return Ok(());
            };
            jump_input.sample(!timing_demo && input.is_key_pressed(KeyCode::Space));
            let pressed = input.is_key_pressed(KeyCode::F3);
            let toggle_debug = pressed && !debug_pressed;
            debug_pressed = pressed;

            let (dx, dy) = if timing_demo { (0.0, 0.0) } else { input.mouse_delta() };
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

            const DEMO_STEPS: usize = 120;
            if timing_demo {
                movement = initial_direction;
                if demo_steps < DEMO_STEPS {
                    demo_frames += 1;
                    demo_elapsed += f64::from(time.time_since_last_frame);
                }
            }
            for _ in 0..steps {
                if timing_demo && demo_steps == DEMO_STEPS {
                    break;
                }
                player.fixed_update(movement, jump_input.take(), simulation::FixedClock::STEP as f32, &colliders);
                if timing_demo {
                    demo_steps += 1;
                    if demo_steps == DEMO_STEPS {
                        println!(
                            "Прогон: 120 шагов, 2 с симуляции, {:.3} м; {:.2} с активных кадров, {:.1} FPS",
                            player.position.distance(start_position),
                            demo_elapsed,
                            f64::from(demo_frames) / demo_elapsed,
                        );
                    }
                }
            }
            if toggle_debug {
                debug_visible = !debug_visible;
                for object in &outlines {
                    context.scene().set_object_visible(*object, debug_visible)?;
                }
            }
            context.scene().set_node_transform(
                corwin,
                Transform {
                    translation: player.position,
                    rotation: player.rotation,
                    ..Transform::default()
                },
            )?;

            if debug_visible {
                context.scene().set_node_transform(
                    player_outline, Transform::from_translation(player.position),
                )?;
            }
            let mut viewpoint = context.scene().viewpoint();
            let direction = Vec3::new(yaw_sin * pitch_cos, pitch_sin, -yaw_cos * pitch_cos);
            viewpoint.position = player.position - direction.normalize() * CAMERA_DISTANCE
                + Vec3::new(0.0, 1.0, 0.0);
            viewpoint.direction = direction;
            context.scene().set_viewpoint(viewpoint)?;
            Ok(())
        }),
        frame_ended: Box::new(move |_, _| {
            limiter.wait();
            Ok(())
        }),
    })?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let limiter = devtools::FrameLimiter::from_env()?;
    let timing_demo = devtools::timing_demo_enabled()?;
    let debug_visible = devtools::collider_debug_enabled()?;
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new("Amber — тестовая сцена", 1280, 720))?;
    let result = build_scene(&mut engine, timing_demo, debug_visible, limiter)
        .and_then(|()| engine.run().map_err(Into::into));
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}
