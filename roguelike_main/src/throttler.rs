use std::sync::mpsc::{channel, Receiver};

use timer::*;

use roguelike_core::constants::*;


struct Throttler {
    guard: Guard,
    tick_receiver: Receiver<usize>
}

impl Throttler {
    pub fn new() -> Throttler {
        // start game tick timer
        let timer = Timer::new();
        let (tick_sender, tick_receiver) = channel();
        let mut ticks: usize = 0;
        let guard = 
            timer.schedule_repeating(chrono::Duration::milliseconds(TIME_BETWEEN_FRAMES_MS), move || {
                tick_sender.send(ticks).unwrap();
                ticks += 1;
            });

        return Throttler {
            guard,
            tick_receiver,
        };
    }

    pub fn block(&self) {
        self.tick_receiver.recv().unwrap();
    }
}

