use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use slint::Weak;
use tts::*;

slint::include_modules!();

#[derive(Clone)]
struct State {
    temperature_goal: f32,
    temperature_level: f32,
    temperature_goal_announced: bool,
}
impl State {
    fn new() -> Self {
        Self {
            temperature_goal: 0.0,
            temperature_level: 0.0,
            temperature_goal_announced: false,
        }
    }

    fn set_temperature_goal(&mut self, goal: f32) {
        self.temperature_goal = goal;
    }

    // todo make decay when temperature goal is lower
    fn update(&mut self) {
        self.temperature_level += self.temperature_goal * 0.005;
        self.temperature_level = self.temperature_level.clamp(0.0, self.temperature_goal);

        if !self.temperature_goal_reached() {
            self.temperature_goal_announced = false;
        }
    }

    fn temperature_goal_reached(&self) -> bool {
        self.temperature_level == self.temperature_goal && self.temperature_level > 0.0
    }
}

fn update_gui(weak_window: Weak<MainWindow>, state: Arc<Mutex<State>>) {
    // hmm, temperature_goal update was prob broken bc this got copied once but not iteratively
    // TODO Fix
    loop {
        let s = state.clone();
        let temperature_level = s.lock().unwrap().temperature_level;
        weak_window
            .upgrade_in_event_loop(move |window| {
                window.set_temperature_level(temperature_level);
            })
            .unwrap();

        thread::sleep(Duration::from_millis(10));
    }
}

fn main() -> Result<(), Error> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut tts = Tts::default()?;

    let state = Arc::new(Mutex::new(State::new()));

    // TODO: delay this after window run
    tts.speak("Powering up Stove", false)?;

    let window = MainWindow::new().unwrap();
    let weak_window: Weak<MainWindow> = window.as_weak();

    let set_power_goal_state = state.clone();
    window.on_temperature_goal_changed(move |goal| {
        set_power_goal_state
            .lock()
            .unwrap()
            .set_temperature_goal(goal);
    });

    let gui_state = state.clone();
    rt.spawn(async move { update_gui(weak_window, gui_state) });

    let tts_state = state.clone();
    rt.spawn(async move {
        loop {
            if tts_state.lock().unwrap().temperature_goal_reached()
                && !tts_state.lock().unwrap().temperature_goal_announced
            {
                let temperature_level = tts_state.lock().unwrap().temperature_level.round();
                let announcement =
                    format!("Power goal reached: {temperature_level} degrees fahrenheit");
                tts.speak(announcement, false).unwrap();
                tts_state.lock().unwrap().temperature_goal_announced = true;
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
