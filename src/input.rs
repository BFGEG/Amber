/// Сохраняет фронт нажатия между кадрами до первого шага симуляции.
#[derive(Default)]
pub struct JumpInput {
    held: bool,
    pending: bool,
}

impl JumpInput {
    pub fn sample(&mut self, held: bool) {
        self.pending |= held && !self.held;
        self.held = held;
    }

    pub fn take(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    pub fn pause(&mut self, held: bool) {
        self.pending = false;
        self.held = held;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_survives_frames_without_steps_and_is_consumed_once() {
        let mut input = JumpInput::default();
        input.sample(true);
        input.sample(false);
        assert!(input.take());
        assert!(!input.take());
    }

    #[test]
    fn holding_does_not_repeat_and_release_allows_another_jump() {
        let mut input = JumpInput::default();
        input.sample(true);
        assert!(input.take());
        input.sample(true);
        assert!(!input.take());
        input.sample(false);
        input.sample(true);
        assert!(input.take());
    }

    #[test]
    fn pause_discards_pending_press_and_does_not_replay_a_held_key() {
        let mut input = JumpInput::default();
        input.sample(true);
        input.pause(true);
        input.sample(true);
        assert!(!input.take());
        input.sample(false);
        input.sample(true);
        assert!(input.take());
    }
}
