mod camera;
mod collider_debug;
mod collision;
mod devtools;
mod input;
mod simulation;
mod skybox;
mod world;

use std::error::Error;
use std::path::Path;

use bfge::math::{Quat, Vec3};
use bfge::{
    DirectionalLight, Engine, EngineApi, EngineEvent, KeyCode, SceneLighting, Transform,
    ViewpointState, WindowDescriptor,
};

const CORWIN_MODEL: &str = concat!(env!("OUT_DIR"), "/corwin.obj");
const CORWIN_WEAPONS: &str = concat!(env!("OUT_DIR"), "/corwin-weapons.obj");
const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cube.obj");
const GROUND_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ground.obj");
const TERRACE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/terrace.obj");
const INITIAL_DIRECTION: Vec3 = Vec3::new(-1.0, -0.46, 0.0);
const TIMING_DIRECTION: Vec3 = Vec3::X;
const MOUSE_SENSITIVITY: f32 = 0.002;

fn build_scene(
    engine: &mut Engine,
    timing_demo: bool,
    physics_demo: bool,
    mut debug_visible: bool,
    mut limiter: devtools::FrameLimiter,
) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let (grounds, ground_model): (&[_], _) = if physics_demo {
        (&[world::GROUND], GROUND_MODEL)
    } else {
        (&[world::LOWER_TERRACE, world::UPPER_TERRACE], TERRACE_MODEL)
    };
    for ground in grounds {
        let node = engine.scene().create_node(root)?;
        engine.scene().set_node_transform(
            node,
            Transform {
                translation: Vec3::new(ground.center.x, ground.bounds().max.y, ground.center.z),
                scale: Vec3::new(ground.size.x, 1.0, ground.size.z),
                ..Transform::default()
            },
        )?;
        engine.load_model(Path::new(ground_model), node)?;
    }

    let corwin = engine.scene().create_node(root)?;
    // Для сравнения FPS оставляем свободную полосу в стороне от препятствий.
    let start_position = if timing_demo {
        world::TIMING_START_POSITION
    } else {
        world::START_POSITION
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
    let corwin_model = engine.load_model(Path::new(CORWIN_MODEL), corwin)?;
    let corwin_weapons = engine.load_model(Path::new(CORWIN_WEAPONS), corwin)?;

    let cube = engine.load_model(Path::new(CUBE_MODEL), root)?;

    let mut blocks = Vec::new();
    if physics_demo {
        blocks.extend(world::diagnostic_blocks().map(|block| (block, [0.25, 0.25, 0.25])));
    } else {
        blocks.extend(grounds.iter().copied().map(|block| {
            // Куб образует боковые откосы. Верх уже закрыт травяной плоскостью.
            let rock = world::StaticBox {
                center: block.center - Vec3::Y * 0.01,
                ..block
            };
            (rock, [0.26, 0.16, 0.09])
        }));
        blocks.extend(world::ASCENT.map(|block| (block, [0.29, 0.41, 0.44])));
        blocks.extend(world::COURSE_WALLS.map(|block| (block, [0.25, 0.29, 0.31])));
        for ground in grounds {
            let path = world::StaticBox {
                center: Vec3::new(
                    ground.center.x,
                    ground.bounds().max.y + 0.012,
                    ground.center.z,
                ),
                size: Vec3::new(ground.size.x - 0.15, 0.018, 8.0),
            };
            blocks.push((path, [0.45, 0.32, 0.17]));
        }
    }
    for (block, color) in blocks {
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
        engine.scene().set_object_color(object, color)?;
    }

    let colliders = if physics_demo {
        world::diagnostic_colliders()
    } else {
        world::course_colliders()
    };
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
    engine.scene().remove_object(cube)?;
    let mut debug_pressed = false;

    engine.scene().set_lighting(SceneLighting {
        ambient: Vec3::splat(0.3),
        directional: DirectionalLight {
            direction: Vec3::new(-0.5, -1.0, -0.7),
            color: Vec3::splat(0.8),
        },
    })?;
    let initial_direction = if timing_demo {
        // Прогон направлен вдоль платформ, как исходный обзор сцены.
        TIMING_DIRECTION
    } else {
        INITIAL_DIRECTION
    }
    .normalize();

    let mut viewpoint = ViewpointState {
        position: Vec3::ZERO,
        direction: initial_direction,
        roll: 0.0,
        fov_y: 55.0_f32.to_radians(),
    };
    let mut follow_camera = camera::FollowCamera::new(start_position);
    let mut corwin_visible = follow_camera.place(
        &mut viewpoint,
        start_position,
        initial_direction,
        &colliders,
    );
    engine
        .scene()
        .set_object_visible(corwin_model, corwin_visible)?;
    engine
        .scene()
        .set_object_visible(corwin_weapons, corwin_visible)?;
    engine.scene().set_viewpoint(viewpoint)?;

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
            let space_held = input.is_key_pressed(KeyCode::Space);
            jump_input.sample(!timing_demo && space_held);
            let pressed = input.is_key_pressed(KeyCode::F3);
            let toggle_debug = pressed && !debug_pressed;
            debug_pressed = pressed;

            let (dx, dy) = if timing_demo { (0.0, 0.0) } else { input.mouse_delta() };
            pitch = camera::clamp_pitch(pitch - dy as f32 * MOUSE_SENSITIVITY);
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
                if player.respawn_if_fallen(start_position, &colliders) {
                    follow_camera = camera::FollowCamera::new(player.position);
                    clock.clear_time();
                    jump_input.pause(space_held);
                    // Остальные шаги кадра не должны сразу сдвинуть персонажа со старта.
                    break;
                }
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
            follow_camera.update_height(player.position, time.time_since_last_frame);
            let visible = follow_camera.place(&mut viewpoint, player.position, direction, &colliders);
            if visible != corwin_visible {
                context.scene().set_object_visible(corwin_model, visible)?;
                context.scene().set_object_visible(corwin_weapons, visible)?;
                corwin_visible = visible;
            }
            context.scene().set_viewpoint(viewpoint)?;
            Ok(())
        }),
        frame_ended: Box::new(move |_, _| {
            limiter.wait();
            Ok(())
        }),
    })?;
    skybox::add_to_scene(engine)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let limiter = devtools::FrameLimiter::from_env()?;
    let timing_demo = devtools::timing_demo_enabled()?;
    let debug_visible = devtools::collider_debug_enabled()?;
    let physics_demo = devtools::physics_demo_enabled()? || timing_demo;
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new("Amber — тестовая сцена", 1280, 720))?;
    let result = build_scene(
        &mut engine,
        timing_demo,
        physics_demo,
        debug_visible,
        limiter,
    )
    .and_then(|()| engine.run().map_err(Into::into));
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}
