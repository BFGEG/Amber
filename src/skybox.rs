use std::error::Error;
use std::path::Path;

use bfge::math::Vec3;
use bfge::{Engine, EngineApi, EngineEvent, Transform};

const MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/skybox.obj");
// В закреплённой версии BFGE дальняя плоскость камеры находится на расстоянии 100.
const RADIUS: f32 = 99.0;

pub fn add_to_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let scene = engine.scene();
    let node = scene.create_node(scene.root_node())?;
    let mut transform = Transform {
        translation: scene.viewpoint().position,
        scale: Vec3::splat(RADIUS),
        ..Transform::default()
    };
    scene.set_node_transform(node, transform)?;
    let sky = engine.load_model(Path::new(MODEL), node)?;
    // Текстура учитывает направленный свет сцены. Компенсируем фоновый свет 0,3.
    engine.scene().set_object_color(sky, [1.0 / 0.3; 3])?;

    engine.add_event(EngineEvent {
        frame_started: Box::new(move |frame, _| {
            let scene = frame.scene();
            let position = scene.viewpoint().position;
            if position != transform.translation {
                transform.translation = position;
                scene.set_node_transform(node, transform)?;
            }
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    Ok(())
}
