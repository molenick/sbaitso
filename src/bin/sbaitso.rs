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
    Off,
    UnderGoal,
    OverGoal,
    AtGoal,
    Announced,
    CoolingOff,
    CooledOff,
    CooledOffAnnounced,
}

enum UserActivity {
    Pending,
    TemperatureGoalSet,
    TemperatureGoalAnnounced,
    BurnerTurnOff,
    BurnerTurnOffAnnounced,
}

struct AppState {
    temperature_goal: f32,
    temperature_level: f32,
    temperature_status: TemperatureStatus,
    tts: Tts,
    user_activity: UserActivity,
}
impl AppState {
    fn new() -> Self {
        Self {
            temperature_goal: 0.0,
            temperature_level: 0.0,
            temperature_status: TemperatureStatus::Off,
            tts: Tts::default().unwrap(),
            user_activity: UserActivity::Pending,
        }
    }

    fn set_temperature_goal(&mut self, goal: f32) {
        // Only update if goal is different
        if self.temperature_goal != goal {
            self.temperature_goal = goal;
            if self.temperature_goal == 0.0 {
                self.user_activity = UserActivity::BurnerTurnOff;
                self.temperature_status = TemperatureStatus::CoolingOff
            } else {
                self.user_activity = UserActivity::TemperatureGoalSet;
            }
        }
    }

    fn calc_temperature_level(&mut self) {
        match self.temperature_status {
            TemperatureStatus::Off => {}
            TemperatureStatus::UnderGoal => {
                self.temperature_level += TEMPERATURE_RATE;
                self.temperature_level = self.temperature_level.clamp(0.0, self.temperature_goal);
            }
            TemperatureStatus::OverGoal => {
                self.temperature_level -= TEMPERATURE_RATE;
                self.temperature_level = self.temperature_level.clamp(0.0, self.temperature_level);
            }
            TemperatureStatus::AtGoal => {}
            TemperatureStatus::Announced => {}
            TemperatureStatus::CoolingOff => {
                self.temperature_level -= TEMPERATURE_RATE;
                self.temperature_level = self.temperature_level.clamp(0.0, self.temperature_level);
            }
            TemperatureStatus::CooledOff => {
                // A bit of a cheat 🤫
                self.temperature_level = 0.0;
            }
            TemperatureStatus::CooledOffAnnounced => {}
        }
    }

    // The bugs live here, mostly.
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

        if self.temperature_goal == 0.0 {
            self.temperature_status = TemperatureStatus::CoolingOff
        }

        if self.temperature_level < 1.0 && self.temperature_goal == 0.0 {
            self.temperature_status = TemperatureStatus::CooledOff
        }

        // If goal and level are zero, ignore other possible states and go to Off
        if self.temperature_level == 0.0 && self.temperature_goal == 0.0 {
            self.temperature_status = TemperatureStatus::Off
        }
    }

    fn announce(&mut self) {
        match self.temperature_status {
            TemperatureStatus::Off => {}
            TemperatureStatus::UnderGoal => {}
            TemperatureStatus::OverGoal => {}
            TemperatureStatus::AtGoal => {
                self.temperature_status = TemperatureStatus::Announced;
                let announcement = format!(
                    "Temp reached: {} degrees fahrenheit",
                    self.temperature_level.round()
                );
                self.tts.speak(announcement, true).unwrap();
            }
            TemperatureStatus::Announced => {}
            TemperatureStatus::CoolingOff => {}
            TemperatureStatus::CooledOff => {
                self.temperature_status = TemperatureStatus::CooledOffAnnounced;

                self.tts.speak("Burner has cooled down", true).unwrap();
            }
            TemperatureStatus::CooledOffAnnounced => {
                self.temperature_status = TemperatureStatus::Off;
            }
        }

        match self.user_activity {
            UserActivity::Pending => {}
            UserActivity::TemperatureGoalSet => {
                self.user_activity = UserActivity::TemperatureGoalAnnounced;
                let announcement = format!(
                    "Temp set: {} degrees fahrenheit",
                    self.temperature_goal.round()
                );
                self.tts.speak(announcement, true).unwrap();
            }
            UserActivity::TemperatureGoalAnnounced => self.user_activity = UserActivity::Pending,
            UserActivity::BurnerTurnOff => {
                self.user_activity = UserActivity::BurnerTurnOffAnnounced;
                self.tts.speak("Burner off, cooling down", true).unwrap();
            }
            UserActivity::BurnerTurnOffAnnounced => self.user_activity = UserActivity::Pending,
        }
    }

    fn update(&mut self) {
        // self.calc_user_activity();
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
