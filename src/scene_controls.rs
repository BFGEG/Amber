use crate::{input::ButtonInput, scene_state::SceneKind};

/// Средства разработки принадлежат приложению и не пересоздаются при загрузке.
pub struct SceneControls {
    primary: SceneKind,
    switch: ButtonInput,
    reload: ButtonInput,
    was_focused: bool,
}

impl SceneControls {
    pub fn new(primary: SceneKind) -> Self {
        Self {
            primary,
            switch: ButtonInput::default(),
            reload: ButtonInput::default(),
            was_focused: false,
        }
    }

    pub fn request(
        &mut self,
        current: SceneKind,
        focused: bool,
        switch: bool,
        reload: bool,
    ) -> Option<SceneKind> {
        let entering = !self.was_focused;
        self.was_focused = focused;
        if !focused || entering {
            self.switch.pause(switch);
            self.reload.pause(reload);
            return None;
        }
        self.switch.sample(switch);
        self.reload.sample(reload);
        let switch = self.switch.take();
        let reload = self.reload.take();
        if switch {
            Some(if current == SceneKind::Demo {
                self.primary
            } else {
                SceneKind::Demo
            })
        } else if reload {
            Some(current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_and_reload_do_not_repeat_while_held() {
        let mut controls = SceneControls::new(SceneKind::Course);
        controls.request(SceneKind::Course, true, false, false);
        assert_eq!(
            controls.request(SceneKind::Course, true, true, false),
            Some(SceneKind::Demo)
        );
        assert_eq!(controls.request(SceneKind::Demo, true, true, false), None);
        controls.request(SceneKind::Demo, true, false, false);
        assert_eq!(
            controls.request(SceneKind::Demo, true, true, false),
            Some(SceneKind::Course)
        );
        controls.request(SceneKind::Course, true, false, false);
        assert_eq!(
            controls.request(SceneKind::Course, true, false, true),
            Some(SceneKind::Course)
        );
        assert_eq!(controls.request(SceneKind::Course, true, false, true), None);
    }

    #[test]
    fn focus_return_does_not_load_from_an_old_held_key() {
        let mut controls = SceneControls::new(SceneKind::Physics);
        controls.request(SceneKind::Physics, true, false, false);
        controls.request(SceneKind::Physics, false, true, true);
        assert_eq!(controls.request(SceneKind::Physics, true, true, true), None);
        controls.request(SceneKind::Physics, true, false, false);
        assert_eq!(
            controls.request(SceneKind::Physics, true, true, false),
            Some(SceneKind::Demo)
        );
        controls.request(SceneKind::Demo, true, false, false);
        assert_eq!(
            controls.request(SceneKind::Demo, true, true, false),
            Some(SceneKind::Physics)
        );
    }
}
