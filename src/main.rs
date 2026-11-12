use std::error::Error;
use std::path::Path;

use bfge::math::Vec3;
use bfge::{
    DirectionalLight, Engine, EngineApi, SceneLighting, Transform, ViewpointState, WindowDescriptor,
};

const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cube.obj");

fn build_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let ground = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        ground,
        Transform {
            translation: Vec3::new(0.0, -0.25, 0.0),
            scale: Vec3::new(20.0, 0.5, 16.0),
            ..Transform::default()
        },
    )?;
    let cube = engine.load_model(Path::new(CUBE_MODEL), ground)?;
    engine.scene().set_object_color(cube, [0.25, 0.32, 0.24])?;

    // Маркер Корвина и пять платформ имеют общий меш, но разные размеры и цвета.
    let blocks = [
        ([-4.0, 0.75, 2.0], [0.7, 1.5, 0.7], [0.85, 0.2, 0.12]),
        ([-2.5, 0.25, 0.0], [1.4, 0.5, 1.4], [0.65, 0.45, 0.17]),
        ([-0.5, 0.5, -0.3], [1.4, 1.0, 1.4], [0.65, 0.45, 0.17]),
        ([1.5, 0.75, -0.6], [1.4, 1.5, 1.4], [0.65, 0.45, 0.17]),
        ([3.5, 1.0, -0.9], [1.4, 2.0, 1.4], [0.65, 0.45, 0.17]),
        ([4.5, 1.25, -2.8], [1.4, 2.5, 1.4], [0.65, 0.45, 0.17]),
    ];
    for (position, size, color) in blocks {
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
        engine.scene().set_object_color(object, color)?;
    }

    engine.scene().set_viewpoint(ViewpointState {
        position: Vec3::new(8.0, 8.0, 13.0),
        direction: Vec3::new(-8.0, -7.5, -13.0),
        fov_y: 55.0_f32.to_radians(),
    })?;
    engine.scene().set_lighting(SceneLighting {
        ambient: Vec3::splat(0.3),
        directional: DirectionalLight {
            direction: Vec3::new(-0.5, -1.0, -0.7),
            color: Vec3::splat(0.8),
        },
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
