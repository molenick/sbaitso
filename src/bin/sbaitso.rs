use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use slint::Weak;
use tts::*;

slint::include_modules!();

const TEMPERATURE_RATE: f32 = 0.1;

#[derive(PartialEq)]
enum TemperatureStatus {
    AtZero,
    UnderGoal,
    OverGoal,
    AtGoal,
    Announced,
}

struct AppState {
    temperature_goal: f32,
    temperature_level: f32,
    temperature_status: TemperatureStatus,
    tts: Tts,
}
impl AppState {
    fn new() -> Self {
        Self {
            temperature_goal: 0.0,
            temperature_level: 0.0,
            temperature_status: TemperatureStatus::AtZero,
            tts: Tts::default().unwrap(),
        }
    }

    fn set_temperature_goal(&mut self, goal: f32) {
        self.temperature_goal = goal;
    }

    fn calc_temperature_level(&mut self) {
        match self.temperature_status {
            TemperatureStatus::AtZero => {}
            TemperatureStatus::UnderGoal => {
                self.temperature_level += TEMPERATURE_RATE;
                self.temperature_level = self.temperature_level.clamp(0.0, self.temperature_goal);
            }
            TemperatureStatus::OverGoal => {
                self.temperature_level -= TEMPERATURE_RATE;
            }
            TemperatureStatus::AtGoal => {}
            TemperatureStatus::Announced => {}
        }
    }

    fn calc_temperature_state(&mut self) {
        if self.temperature_level == self.temperature_goal
            && self.temperature_status != TemperatureStatus::Announced
        {
            self.temperature_status = TemperatureStatus::AtGoal;
        }

        if self.temperature_level > self.temperature_goal {
            self.temperature_status = TemperatureStatus::OverGoal;
        }

        if self.temperature_level < self.temperature_goal {
            self.temperature_status = TemperatureStatus::UnderGoal;
        }

        if self.temperature_level == 0.0 && self.temperature_goal == 0.0 {
            self.temperature_status = TemperatureStatus::AtZero
        }
    }

    fn announce(&mut self) {
        match self.temperature_status {
            TemperatureStatus::AtZero => {}
            TemperatureStatus::UnderGoal => {}
            TemperatureStatus::OverGoal => {}
            TemperatureStatus::AtGoal => {
                let announcement = format!(
                    "Temperature goal reached: {} degrees fahrenheit",
                    self.temperature_level.round()
                );
                self.tts.speak(announcement, true).unwrap();
                self.temperature_status = TemperatureStatus::Announced;
            }
            TemperatureStatus::Announced => {}
        }
    }

    fn update(&mut self) {
        self.calc_temperature_state();
        self.calc_temperature_level();
        self.announce();
    }
}

fn update_gui(weak_window: Weak<MainWindow>, state: Arc<Mutex<AppState>>) {
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
    let state = Arc::new(Mutex::new(AppState::new()));
    let window = MainWindow::new().unwrap();
    let weak_window: Weak<MainWindow> = window.as_weak();

    let set_temperature_goal_state = state.clone();
    window.on_temperature_goal_changed(move |goal| {
        set_temperature_goal_state
            .lock()
            .unwrap()
            .set_temperature_goal(goal);
    });

    let gui_state = state.clone();
    rt.spawn(async move { update_gui(weak_window, gui_state) });

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
