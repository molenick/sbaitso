use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use slint::Weak;
use tts::*;

slint::include_modules!();

#[cfg(target_os = "macos")]
use cocoa_foundation::base::id;
#[cfg(target_os = "macos")]
use cocoa_foundation::foundation::NSRunLoop;
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

#[derive(Clone)]
struct State {
    power_goal: f32,
    power_level: f32,
    power_goal_reached: bool,
    power_goal_announced: bool,
}
impl State {
    fn new() -> Self {
        Self {
            power_goal: 0.0,
            power_level: 0.0,
            power_goal_reached: false,
            power_goal_announced: false,
        }
    }

    fn set_power_goal(&mut self, goal: f32) {
        self.power_goal = goal;
    }

    // todo make decay when power goal is lower
    fn update(&mut self) {
        self.power_level += self.power_goal * 0.005;
        self.power_level = self.power_level.clamp(0.0, self.power_goal);

        self.power_goal_reached = self.power_goal_reached();
        if !self.power_goal_reached {
            self.power_goal_announced = false;
        }
    }

    fn power_goal_reached(&self) -> bool {
        self.power_level == self.power_goal && self.power_level > 0.0
    }
}

fn update_gui(weak_window: Weak<MainWindow>, state: Arc<Mutex<State>>) {
    // hmm, power_goal update was prob broken bc this got copied once but not iteratively
    // TODO Fix
    loop {
        let s = state.clone();
        let power_level = s.lock().unwrap().power_level;
        weak_window
            .upgrade_in_event_loop(move |window| {
                window.set_power_level(power_level);
            })
            .unwrap();

        thread::sleep(Duration::from_millis(10));
    }
}

fn main() -> Result<(), Error> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut tts = Tts::default()?;
    // provide_nsloop(&rt);

    let state = Arc::new(Mutex::new(State::new()));

    // TODO: delay this after window run
    tts.speak("Powering up Stove", false)?;

    let window = MainWindow::new().unwrap();
    let weak_window: Weak<MainWindow> = window.as_weak();

    let gui_state = state.clone();
    rt.spawn(async move { update_gui(weak_window, gui_state) });

    let set_power_goal_state = state.clone();
    window.on_power_goal_changed(move |goal| {
        set_power_goal_state.lock().unwrap().set_power_goal(goal);
    });

    let tts_state = state.clone();
    rt.spawn(async move {
        loop {
            // let mut s = tts_state.lock().unwrap();

            if tts_state.lock().unwrap().power_goal_reached
                && !tts_state.lock().unwrap().power_goal_announced
            {
                tts.speak("Power goal reached", false).unwrap();
                tts_state.lock().unwrap().power_goal_announced = true;
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    // calculate new state
    rt.spawn(async move {
        loop {
            state.lock().unwrap().update();
            thread::sleep(Duration::from_millis(10));
        }
    });

    window.run().unwrap();

    Ok(())
}

#[cfg(target_os = "macos")]
fn provide_nsloop(rt: &tokio::runtime::Runtime) {
    rt.spawn(async move {
        {
            let run_loop: id = unsafe { NSRunLoop::currentRunLoop() };
            unsafe {
                let _: () = msg_send![run_loop, run];
            }
        }
    });
}
