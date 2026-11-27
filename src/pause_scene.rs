use bfge::math::{self, Quat, Vec3};
use bfge::{
    FrameContext, MouseButton, SceneApi, SceneError, SceneNodeId, SceneObjectId, Transform,
    WindowSize,
};

use crate::input::ButtonInput;
#[path = "pause_layout.rs"]
mod layout;
use layout::{CARD_HEIGHT, CARD_WIDTH, EXIT_BOUNDS};
const CARD_DISTANCE: f32 = 0.25;
const CURSOR_DISTANCE: f32 = 0.24;

pub const CARD_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/pause-card.obj");
pub const CURSOR_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/pause-cursor.obj");

pub const HOVER_MODEL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/pause-card-hover.obj"
);

/// Карточка с заранее отрисованным текстом, закреплённая перед камерой.
/// Модель загружается один раз; пауза меняет только transform и видимость.
pub struct PauseOverlay {
    root: SceneNodeId,
    cards: [SceneObjectId; 2],
    cursor_node: SceneNodeId,
    cursor: SceneObjectId,
    hovered: bool,
    input: MenuInput,
    visible: bool,
}

impl PauseOverlay {
    pub fn new(
        scene: &mut dyn SceneApi,
        cards: [SceneObjectId; 2],
        cursor: SceneObjectId,
    ) -> Result<Self, SceneError> {
        let root = scene.create_node(scene.root_node())?;
        for card in cards {
            scene.reparent_object(card, root)?;
            scene.set_object_visible(card, false)?;
        }
        let cursor_node = scene.create_node(scene.root_node())?;
        scene.reparent_object(cursor, cursor_node)?;
        scene.set_object_visible(cursor, false)?;
        Ok(Self {
            root,
            cards,
            cursor_node,
            cursor,
            hovered: false,
            input: MenuInput::default(),
            visible: false,
        })
    }

    pub fn interact(
        &mut self,
        context: &mut FrameContext<'_>,
        paused: bool,
    ) -> Result<bool, SceneError> {
        let active = paused && context.input().is_focused();
        let cursor = context.input().cursor_position();
        let size = context.window_size();
        let hovered = active && hit_exit(cursor, size, context.scene().viewpoint().fov_y);
        let clicked = self.input.exit(
            active,
            hovered,
            context.input().is_mouse_pressed(MouseButton::Left),
        );
        self.synchronize(context.scene(), paused)?;
        if paused {
            self.update_cursor(context.scene(), active, cursor, size)?;
        }
        if self.hovered != hovered {
            self.hovered = hovered;
            for (index, card) in self.cards.iter().enumerate() {
                context
                    .scene()
                    .set_object_visible(*card, paused && index == usize::from(hovered))?;
            }
        }
        Ok(clicked)
    }

    fn update_cursor(
        &self,
        scene: &mut dyn SceneApi,
        active: bool,
        (x, y): (f64, f64),
        size: WindowSize,
    ) -> Result<(), SceneError> {
        let visible = active
            && (0.0..f64::from(size.width)).contains(&x)
            && (0.0..f64::from(size.height)).contains(&y);
        if visible {
            let view = scene.viewpoint();
            let inverse_view = math::view(view.position, view.direction, view.roll).inverse();
            // Плоскость указателя ближе карточки, а её масштаб соответствует пикселям окна.
            let units_per_pixel =
                2.0 * CURSOR_DISTANCE * (view.fov_y * 0.5).tan() / size.height as f32;
            let position = Vec3::new(
                (x as f32 - size.width as f32 * 0.5) * units_per_pixel,
                (size.height as f32 * 0.5 - y as f32) * units_per_pixel,
                -CURSOR_DISTANCE,
            );
            scene.set_node_transform(
                self.cursor_node,
                Transform {
                    translation: inverse_view.transform_point3(position),
                    rotation: Quat::from_mat4(&inverse_view),
                    scale: Vec3::splat(units_per_pixel),
                },
            )?;
        }
        scene.set_object_visible(self.cursor, visible)
    }

    pub fn synchronize(
        &mut self,
        scene: &mut dyn SceneApi,
        paused: bool,
    ) -> Result<(), SceneError> {
        if self.visible != paused {
            if paused {
                let view = scene.viewpoint();
                let inverse_view = math::view(view.position, view.direction, view.roll).inverse();
                scene.set_node_transform(
                    self.root,
                    Transform {
                        translation: view.position + view.direction.normalize() * CARD_DISTANCE,
                        rotation: Quat::from_mat4(&inverse_view),
                        ..Transform::default()
                    },
                )?;
            }
            for (index, card) in self.cards.iter().enumerate() {
                scene.set_object_visible(*card, paused && index == usize::from(self.hovered))?;
            }
            if !paused {
                scene.set_object_visible(self.cursor, false)?;
            }
            self.visible = paused;
        }
        Ok(())
    }
}

fn hit_exit(cursor: (f64, f64), size: WindowSize, fov: f32) -> bool {
    if size.width == 0
        || size.height == 0
        || !fov.is_finite()
        || fov <= 0.0
        || fov >= std::f32::consts::PI
    {
        return false;
    }
    let scale = f64::from(size.height) / (2.0 * f64::from((fov * 0.5).tan() * CARD_DISTANCE));
    let x = (cursor.0 - f64::from(size.width) * 0.5) / scale / f64::from(CARD_WIDTH) + 0.5;
    let y = 0.5 - (f64::from(size.height) * 0.5 - cursor.1) / scale / f64::from(CARD_HEIGHT);
    let [left, top, right, bottom] = EXIT_BOUNDS.map(f64::from);
    x >= left && x <= right && y >= top && y <= bottom
}

#[derive(Default)]
struct MenuInput {
    click: ButtonInput,
    was_active: bool,
}

impl MenuInput {
    fn exit(&mut self, active: bool, hovered: bool, held: bool) -> bool {
        let entering = !self.was_active;
        self.was_active = active;
        if !active || entering {
            self.click.pause(held);
            return false;
        }
        self.click.sample(held);
        let clicked = self.click.take();
        clicked && hovered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor_at(x: f32, y: f32, size: WindowSize, fov: f32) -> (f64, f64) {
        let scale = f64::from(size.height) / (2.0 * f64::from((fov * 0.5).tan() * CARD_DISTANCE));
        (
            f64::from(size.width) * 0.5 + f64::from((x - 0.5) * CARD_WIDTH) * scale,
            f64::from(size.height) * 0.5 - f64::from((0.5 - y) * CARD_HEIGHT) * scale,
        )
    }

    #[test]
    fn button_hit_tracks_resize_pixel_density_and_fov() {
        let [left, top, right, bottom] = EXIT_BOUNDS;
        for size in [
            WindowSize::new(1280, 720),
            WindowSize::new(720, 1280),
            WindowSize::new(2560, 1440),
        ] {
            for fov in [55.0_f32.to_radians(), 90.0_f32.to_radians()] {
                let center = cursor_at((left + right) * 0.5, (top + bottom) * 0.5, size, fov);
                assert!(hit_exit(center, size, fov));
                for (x, y) in [
                    (left - 0.01, top),
                    (right + 0.01, top),
                    (left, top - 0.01),
                    (left, bottom + 0.01),
                ] {
                    assert!(!hit_exit(cursor_at(x, y, size, fov), size, fov));
                }
                assert!(!hit_exit((0.0, 0.0), size, fov));
            }
        }
        assert!(!hit_exit((0.0, 0.0), WindowSize::new(0, 0), 1.0));
    }

    #[test]
    fn opening_or_refocusing_menu_does_not_replay_held_click() {
        let mut input = MenuInput::default();
        assert!(!input.exit(false, true, true));
        assert!(!input.exit(true, true, true));
        assert!(!input.exit(true, true, true));
        assert!(!input.exit(true, true, false));
        assert!(input.exit(true, true, true));
        assert!(!input.exit(true, true, true));
        assert!(!input.exit(false, true, true));
        assert!(!input.exit(true, true, true));
    }

    #[test]
    fn dragging_held_click_into_button_does_not_exit() {
        let mut input = MenuInput::default();
        input.exit(true, false, false);
        assert!(!input.exit(true, false, true));
        assert!(!input.exit(true, true, true));
        input.exit(true, true, false);
        assert!(input.exit(true, true, true));
    }
}
