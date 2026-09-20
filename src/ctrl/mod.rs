use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};
use bitflags::bitflags;

pub type ControllerState = Arc<AtomicU8>;

pub fn create_controller_state() -> ControllerState {
    Arc::new(AtomicU8::new(0))
}

pub fn press(state: &ControllerState, button: Buttons) {
    state.fetch_or(button.bits(), Ordering::Relaxed);
}

pub fn release(state: &ControllerState, button: Buttons) {
    state.fetch_and(!button.bits(), Ordering::Relaxed);
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct Buttons: u8 {
        const A      = 1 << 0;
        const B      = 1 << 1;
        const SELECT = 1 << 2;
        const START  = 1 << 3;
        const UP     = 1 << 4;
        const DOWN   = 1 << 5;
        const LEFT   = 1 << 6;
        const RIGHT  = 1 << 7;
    }
}

pub struct Controller {
    state: ControllerState,
    pub strobe: bool,
    pub index: u8,
    latched: Buttons,
}

impl Controller {
    pub fn new(state: ControllerState) -> Rc<RefCell<Controller>> {
        Rc::new(RefCell::new(Self {
            state,
            strobe: false,
            index: 0,
            latched: Buttons::from_bits_truncate(0),
        }))
    }

    pub fn read(&mut self) -> u8 {
        if self.index > 7 {
            return 0x41;
        }
        let response = if self.strobe {
            // While strobe is high, reads always return the current A button state.
            self.get_state().bits() & 0x01
        } else {
            let bit = (self.latched.bits() & (1 << self.index)) >> self.index;
            self.index += 1;
            bit
        };
        // Bit 6 is typically high on NES controller reads; other bits are open bus.
        response | 0x40
    }

    pub fn write(&mut self, v: u8) {
        let prev_strobe = self.strobe;
        let new_strobe = v & 1 == 1;
        if new_strobe {
            // While strobe high, continually latch current state
            self.latched = self.get_state();
            self.index = 0;
        } else if prev_strobe {
            // Hardware behavior: latch once on 1->0 transition.
            self.latched = self.get_state();
            self.index = 0;
        }
        self.strobe = new_strobe;
    }

    fn get_state(&self) -> Buttons {
        Buttons::from_bits_truncate(self.state.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strobe_high_returns_live_a_button() {
        let state = create_controller_state();
        let controller = Controller::new(state.clone());
        let mut pad = controller.borrow_mut();

        pad.write(1);
        assert_eq!(pad.read() & 1, 0);

        press(&state, Buttons::A);
        assert_eq!(pad.read() & 1, 1);

        release(&state, Buttons::A);
        assert_eq!(pad.read() & 1, 0);
    }

    #[test]
    fn latch_happens_on_falling_edge_only() {
        let state = create_controller_state();
        press(&state, Buttons::A);
        let controller = Controller::new(state.clone());
        let mut pad = controller.borrow_mut();

        pad.write(1);
        pad.write(0);
        assert_eq!(pad.read() & 1, 1); // A from latched state

        // Change state after latching.
        release(&state, Buttons::A);
        // Additional writes with 0 must not relatch.
        pad.write(0);
        assert_eq!(pad.read() & 1, 0); // B bit from old latched state
    }
}
