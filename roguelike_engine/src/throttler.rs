use std::thread;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};


pub struct Throttler {
    tick_receiver: Receiver<usize>,
    thread: thread::JoinHandle<()>,
}

impl Throttler {
    pub fn new(tick_length: Duration) -> Throttler {
        // start game tick timer
        let (tick_sender, tick_receiver) = channel();
        let mut ticks: usize = 0;

        let mut last_tick = Instant::now();
        let tick_length = tick_length;
        let mut tick_error = Duration::from_secs(0);

        let thread = thread::spawn(move ||{
            loop {
                let sleep_time =
                    tick_length.checked_sub(tick_error)
                               .map_or(Duration::from_secs(0), |sleep_time| sleep_time);
                thread::sleep(sleep_time);

                let send_result = tick_sender.send(ticks);

                if send_result.is_err() {
                    break;
                }

                let current_time = Instant::now();
                tick_error = current_time.duration_since(last_tick)
                                         .checked_sub(tick_length)
                                         .unwrap_or(Duration::from_secs(0));
                last_tick = current_time; 

                ticks += 1;
            }
        });

        return Throttler {
            tick_receiver,
            thread,
        };
    }

    pub fn wait(&self) {
        self.tick_receiver.recv().unwrap();
    }
}
