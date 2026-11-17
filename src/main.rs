use std::error::Error;
use std::path::Path;

use bfge::math::Vec3;
use bfge::{
    DirectionalLight, Engine, EngineApi, SceneLighting, Transform, ViewpointState, WindowDescriptor,
};

const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cube.obj");
const GROUND_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ground.obj");

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
    engine.scene().set_node_transform(
        corwin,
        Transform {
            translation: Vec3::new(-4.0, 0.75, 2.0),
            scale: Vec3::new(0.7, 1.5, 0.7),
            ..Transform::default()
        },
    )?;
    let cube = engine.load_model(Path::new(CUBE_MODEL), corwin)?;
    engine.scene().set_object_color(cube, [0.85, 0.2, 0.12])?;

    // Маркер Корвина и пять платформ используют общий меш куба.
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
        position: Vec3::new(-10.0, 6.0, 2.0),
        direction: Vec3::new(1.0, -0.56, 0.0),
        fov_y: 48.0_f32.to_radians(),
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
