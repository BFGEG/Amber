mod camera;
mod collider_debug;
mod collision;
mod devtools;
mod input;
mod pause;
mod pause_scene;
mod recovery;
mod scene_controls;
mod scene_state;
mod scene_visuals;
mod section;
mod section_scene;
mod simulation;
mod skybox;
mod world;
mod zone;
mod zone_scene;

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
const CHECKPOINT_LIGHT: &str = concat!(env!("OUT_DIR"), "/checkpoint-light.obj");
const MOUSE_SENSITIVITY: f32 = 0.002;
const HAZARD_HOLD_SECONDS: f32 = 0.15;

fn build_scene(
    engine: &mut Engine,
    timing_demo: bool,
    physics_demo: bool,
    mut debug_visible: bool,
    mut limiter: devtools::FrameLimiter,
) -> Result<(), Box<dyn Error>> {
    let kind = if physics_demo {
        scene_state::SceneKind::Physics
    } else {
        scene_state::SceneKind::Course
    };
    let mut state = scene_state::SceneState::new(kind, timing_demo);
    let physics_demo = state.kind == scene_state::SceneKind::Physics;
    let root = engine.scene().root_node();
    let (grounds, ground_model): (&[_], _) = if physics_demo {
        (&[world::GROUND], GROUND_MODEL)
    } else {
        (&[world::LOWER_TERRACE, world::UPPER_TERRACE], TERRACE_MODEL)
    };
    let mut primary_objects = Vec::new();
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
        primary_objects.push(engine.load_model(Path::new(ground_model), node)?);
    }

    let corwin = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        corwin,
        Transform {
            translation: state.player.position,
            rotation: Quat::IDENTITY,
            ..Transform::default()
        },
    )?;
    let corwin_model = engine.load_model(Path::new(CORWIN_MODEL), corwin)?;
    let corwin_weapons = engine.load_model(Path::new(CORWIN_WEAPONS), corwin)?;

    let cube = engine.load_model(Path::new(CUBE_MODEL), root)?;

    let zones = state.kind.event_zones();
    let zone_visuals = zone_scene::add_to_scene(engine.scene(), root, cube, &zones)?;
    let checkpoint_visuals = if physics_demo {
        None
    } else {
        let light = engine.load_model(Path::new(CHECKPOINT_LIGHT), root)?;
        Some(zone_scene::add_checkpoint_visuals(
            engine.scene(),
            root,
            cube,
            light,
            world::CHECKPOINT_ZONE,
        )?)
    };

    let demo_zone = state.kind.section_zone();
    let section_visuals = if let Some(bounds) = demo_zone {
        Some(section_scene::add_to_scene(
            engine.scene(),
            root,
            cube,
            bounds,
        )?)
    } else {
        None
    };

    let hazards = state.kind.hazard_zones();
    let hazard_visuals = zone_scene::add_hazard_visuals(engine.scene(), root, cube, &hazards)?;

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
        primary_objects.push(object);
    }

    let mut primary_outlines = Vec::new();
    for bounds in &state.colliders {
        primary_outlines.extend(collider_debug::create_outline(
            engine.scene(),
            root,
            cube,
            *bounds,
            [0.1, 1.0, 0.2],
        )?);
    }
    let player_outline = engine.scene().create_node(root)?;
    let player_outlines = collider_debug::create_outline(
        engine.scene(),
        player_outline,
        cube,
        collision::player_bounds(Vec3::ZERO),
        [1.0, 0.65, 0.05],
    )?;
    engine.scene().set_node_transform(
        player_outline,
        Transform::from_translation(state.player.position),
    )?;
    let demo_floor = world::DEMO_FLOOR;
    let demo_node = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        demo_node,
        Transform {
            translation: demo_floor.center,
            scale: demo_floor.size,
            ..Transform::default()
        },
    )?;
    let demo_object = engine.scene().clone_object(cube, demo_node)?;
    engine
        .scene()
        .set_object_color(demo_object, [0.08, 0.25, 0.65])?;
    let demo_outlines = collider_debug::create_outline(
        engine.scene(),
        root,
        cube,
        demo_floor.bounds(),
        [0.1, 1.0, 0.2],
    )?;
    let mut visuals = scene_visuals::SceneVisuals {
        primary_objects,
        demo_objects: vec![demo_object],
        primary_outlines,
        demo_outlines,
        player_outlines,
        zones: zone_visuals,
        hazards: hazard_visuals,
        checkpoint: checkpoint_visuals,
        section: section_visuals,
    };
    visuals.activate(engine.scene(), &state, debug_visible)?;
    let pause_card = engine.load_model(Path::new(pause_scene::CARD_MODEL), root)?;
    let hover_card = engine.load_model(Path::new(pause_scene::HOVER_MODEL), root)?;
    let mut pause_overlay =
        pause_scene::PauseOverlay::new(engine.scene(), [pause_card, hover_card])?;
    engine.scene().remove_object(cube)?;

    engine.scene().set_lighting(SceneLighting {
        ambient: Vec3::splat(0.3),
        directional: DirectionalLight {
            direction: Vec3::new(-0.5, -1.0, -0.7),
            color: Vec3::splat(0.8),
        },
    })?;
    let mut viewpoint = ViewpointState {
        position: Vec3::ZERO,
        direction: state.initial_direction,
        roll: 0.0,
        fov_y: 55.0_f32.to_radians(),
    };
    let mut corwin_visible = state.follow_camera.place(
        &mut viewpoint,
        state.start_position,
        state.initial_direction,
        &state.colliders,
    );
    engine
        .scene()
        .set_object_visible(corwin_model, corwin_visible)?;
    engine
        .scene()
        .set_object_visible(corwin_weapons, corwin_visible)?;
    engine.scene().set_viewpoint(viewpoint)?;

    let mut scene_controls = scene_controls::SceneControls::new(kind);
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let request = scene_controls.request(
                state.kind, context.input().is_focused(),
                context.input().is_key_pressed(KeyCode::F5), context.input().is_key_pressed(KeyCode::F6),
            );
            if let Some(kind) = request {
                state = scene_state::SceneState::new(kind, timing_demo);
                visuals.activate(context.scene(), &state, debug_visible)?;
                pause_overlay.synchronize(context.scene(), false)?;
                context.scene().set_node_transform(corwin, Transform {
                    translation: state.player.position, rotation: state.player.rotation, ..Transform::default()
                })?;
                context.scene().set_node_transform(player_outline, Transform::from_translation(state.player.position))?;
                let mut viewpoint = context.scene().viewpoint();
                corwin_visible = state.follow_camera.place(&mut viewpoint, state.player.position, state.initial_direction, &state.colliders);
                context.scene().set_object_visible(corwin_model, corwin_visible)?;
                context.scene().set_object_visible(corwin_weapons, corwin_visible)?;
                context.scene().set_viewpoint(viewpoint)?;
                return Ok(());
            }
            let focused = context.input().is_focused();
            let escape_held = context.input().is_key_pressed(KeyCode::Escape);
            let was_paused = state.pause.is_paused();
            let steps = state.pause.frame_steps(&mut state.clock, time.time_since_last_frame, focused, escape_held);
            if pause_overlay.interact(context, state.pause.is_paused())? {
                context.request_exit();
                return Ok(());
            }
            if was_paused && !state.pause.is_paused() && state.player.grounded {
                // Отпущенное на паузе направление не должно оставлять инерцию на земле.
                state.player.horizontal_velocity = Vec3::ZERO;
            }
            let input = context.input();
            let Some(steps) = steps else {
                state.jump_input.pause(input.is_key_pressed(KeyCode::Space));
                state.return_input.pause(input.is_key_pressed(KeyCode::KeyR));
                state.debug_pressed = input.is_key_pressed(KeyCode::F3);
                return Ok(());
            };
            let space_held = input.is_key_pressed(KeyCode::Space);
            state.jump_input.sample(!state.timing_demo && space_held);
            let return_held = input.is_key_pressed(KeyCode::KeyR);
            state.return_input.sample(!state.timing_demo && return_held);
            let pressed = input.is_key_pressed(KeyCode::F3);
            let toggle_debug = pressed && !state.debug_pressed;
            state.debug_pressed = pressed;

            let (dx, dy) = if state.timing_demo { (0.0, 0.0) } else { input.mouse_delta() };
            state.pitch = camera::clamp_pitch(state.pitch - dy as f32 * MOUSE_SENSITIVITY);
            state.yaw += dx as f32 * MOUSE_SENSITIVITY;

            let (yaw_sin, yaw_cos) = state.yaw.sin_cos();

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
            let precise = !state.timing_demo
                && (input.is_key_pressed(KeyCode::ShiftLeft)
                    || input.is_key_pressed(KeyCode::ShiftRight));
            let speed_mode = if precise {
                simulation::SpeedMode::Precise
            } else {
                simulation::SpeedMode::Normal
            };

            const DEMO_STEPS: usize = 120;
            if state.timing_demo {
                movement = state.initial_direction;
                if state.demo_steps < DEMO_STEPS {
                    state.demo_frames += 1;
                    state.demo_elapsed += f64::from(time.time_since_last_frame);
                }
            }
            let mut zone_events = Vec::new();
            let mut checkpoint_activated = false;
            let mut restore_requested = state.return_input.take();
            if !restore_requested && state.hazard_hold > 0.0 {
                state.hazard_hold = (state.hazard_hold - time.time_since_last_frame).max(0.0);
                restore_requested = state.hazard_hold == 0.0;
            } else if !restore_requested {
                for _ in 0..steps {
                    if state.timing_demo && state.demo_steps == DEMO_STEPS {
                        break;
                    }
                    state.player.fixed_update(
                        movement,
                        state.jump_input.take(),
                        speed_mode,
                        simulation::FixedClock::STEP as f32,
                        &state.colliders,
                    );
                    let player_bounds = collision::player_bounds(state.player.position);
                    let hazard_contact =
                        state.hazard_set.update(state.player.position).into_iter().find_map(|event| {
                            match event {
                                zone::ZoneEvent::Entered(id) | zone::ZoneEvent::Stayed(id) => {
                                    Some(id)
                                }
                                _ => None,
                            }
                        });
                    if state.player.is_below_fall_limit() {
                        restore_requested = true;
                        break;
                    }
                    if let Some(id) = hazard_contact {
                        state.hazard_hold = HAZARD_HOLD_SECONDS;
                        state.hazard_alert = Some(id);
                        visuals.hazards.set_alert(context.scene(), id, true)?;
                        state.clock.clear_time();
                        state.jump_input.pause(space_held);
                        break;
                    }
                    let events = state.checkpoint_set.update(player_bounds.center());
                    if events.contains(&zone::ZoneEvent::Triggered(zone::ZoneId(0))) {
                        checkpoint_activated |= state.recovery.activate(recovery::CheckpointPose {
                            position: world::CHECKPOINT_POSITION,
                            yaw: state.yaw,
                            pitch: state.pitch,
                        });
                    }
                    state.section.update(player_bounds.center());
                    zone_events.extend(state.zone_set.update(player_bounds.center()));
                    if state.timing_demo {
                        state.demo_steps += 1;
                        if state.demo_steps == DEMO_STEPS {
                            println!(
                                "Прогон: 120 шагов, 2 с симуляции, {:.3} м; {:.2} с активных кадров, {:.1} FPS",
                                state.player.position.distance(state.start_position),
                                state.demo_elapsed,
                                f64::from(state.demo_frames) / state.demo_elapsed,
                            );
                        }
                    }
                }
            }
            if restore_requested {
                let pose = state.recovery.restore(&mut state.section, &mut state.player, &state.colliders);
                state.yaw = pose.yaw;
                state.pitch = pose.pitch;
                state.follow_camera = camera::FollowCamera::new(state.player.position);
                state.clock.clear_time();
                state.jump_input.pause(space_held);
                state.return_input.pause(return_held);
                state.hazard_hold = 0.0;
                if let Some(id) = state.hazard_alert.take() {
                    visuals.hazards.set_alert(context.scene(), id, false)?;
                }
                let player_center = collision::player_bounds(state.player.position).center();
                state.zone_set.reset_at(player_center);
                state.hazard_set.reset_at(state.player.position);
                zone_events.clear();
                visuals.zones.synchronize(context.scene(), &state.zone_set)?;
            }
            if let Some(visuals) = &mut visuals.section {
                visuals.synchronize(context.scene(), &state.section)?;
            }
            visuals.zones.apply(context.scene(), &zone_events)?;
            if checkpoint_activated && let Some(visuals) = &mut visuals.checkpoint {
                visuals.activate(context.scene())?;
            }
            if toggle_debug {
                debug_visible = !debug_visible;
                visuals.set_debug(context.scene(), state.kind, debug_visible)?;
            }
            context.scene().set_node_transform(
                corwin,
                Transform {
                    translation: state.player.position,
                    rotation: state.player.rotation,
                    ..Transform::default()
                },
            )?;

            if debug_visible {
                context.scene().set_node_transform(
                    player_outline, Transform::from_translation(state.player.position),
                )?;
            }
            let mut viewpoint = context.scene().viewpoint();
            // После возврата используем восстановленный ракурс уже в этом кадре.
            let (yaw_sin, yaw_cos) = state.yaw.sin_cos();
            let (pitch_sin, pitch_cos) = state.pitch.sin_cos();
            let direction = Vec3::new(yaw_sin * pitch_cos, pitch_sin, -yaw_cos * pitch_cos);
            state.follow_camera.update_height(state.player.position, time.time_since_last_frame);
            let visible = state.follow_camera.place(&mut viewpoint, state.player.position, direction, &state.colliders);
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
