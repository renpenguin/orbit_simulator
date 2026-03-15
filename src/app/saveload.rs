use std::{fmt::Display, str::FromStr};

use crate::{
    App,
    app::simulation::{Planet, Vec2},
};

#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, rc::Rc, thread};
#[cfg(target_arch = "wasm32")]
pub struct Task<T>(Rc<Cell<Option<thread::Result<T>>>>);

#[cfg(target_arch = "wasm32")]
impl<T: 'static> Task<T> {
    pub fn spawn<F: 'static + Future<Output = T>>(future: F) -> Self {
        use futures::future::FutureExt;

        let sender = Rc::new(Cell::new(None));
        let receiver = sender.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let future = std::panic::AssertUnwindSafe(future).catch_unwind();
            sender.set(Some(future.await));
        });
        Self(receiver)
    }
    pub fn take_output(&self) -> Option<thread::Result<T>> {
        self.0.take()
    }
}

fn parse_next<T>(numbers: &mut dyn Iterator<Item = &str>) -> Result<T, String>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    numbers
        .next()
        .ok_or_else(|| String::from("reached EOF early - expected another number"))
        .and_then(|word| word.parse::<T>().map_err(|err| format!("{err} ({word})")))
}

impl App {
    /// Ask the user to choose a new save location and store it in `App.save_location`
    #[cfg(not(target_arch = "wasm32"))]
    pub fn choose_new_save_location(&mut self) {
        // Returns None if user cancels operation
        let handle = rfd::FileDialog::new()
            .set_can_create_directories(true)
            .set_title("Save simulation to file")
            .set_file_name("orbit_simulation.sim")
            .save_file();

        if handle.is_some() {
            self.save_file = handle;
        }
    }

    /// Generate save data to save to a file (platform-agnostic)
    fn generate_save_data(&self) -> String {
        let mut save = String::new();

        // writes line "<viewport x> <viewport y> <viewport zoom>" to `save`
        save.push_str(&format!(
            "{} {} {}\n",
            self.viewport_focus.x, self.viewport_focus.y, self.viewport_zoom,
        ));
        // writes line "<tick rate> <planets count>" to `save`
        save.push_str(&format!(
            "{} {}\n",
            self.simulation.tick_rate,
            self.simulation.planets.len(),
        ));

        // writes planet line for each planet: "<px> <py> <vx> <vy> <locked>"
        for planet in self.simulation.get_planets() {
            let lock_num = i32::from(planet.locked);

            save.push_str(&format!(
                "{} {} {} {} {} {}\n",
                planet.pos.x, planet.pos.y, planet.vel.x, planet.vel.y, planet.mass, lock_num
            ));
        }

        save
    }

    /// Save the simulation to the save location stored in `App.save_location`
    #[cfg(not(target_arch = "wasm32"))]
    fn save_native(&mut self) {
        use std::{fs::File, io::Write as _};

        let Some(save_path) = &self.save_file else {
            return;
        };

        let save_data = self.generate_save_data();

        let r = File::create(save_path).and_then(|mut f| f.write_all(save_data.as_bytes()));
        if let Err(err) = r {
            self.error_message = Some(err.to_string());
        }
    }

    /// Save the simulation to the browser's chosen folder (`Downloads by default`)
    #[cfg(target_arch = "wasm32")]
    fn save_web(&mut self) {
        let save_data = self.generate_save_data();

        async_std::task::block_on(async move {
            let file: Option<rfd::FileHandle> = rfd::AsyncFileDialog::new()
                .set_file_name("orbit_simulation.sim")
                .save_file()
                .await;

            let error = match file {
                Some(f) => f
                    .write(save_data.as_bytes())
                    .await
                    .map_err(|err| err.to_string()), // If file write error
                None => Err(String::from("Failed to save file")), // If file picker returned none
            };

            if let Err(error_message) = error {
                rfd::AsyncMessageDialog::new()
                    .set_title("Error")
                    .set_description(error_message)
                    .show()
                    .await;
            }
        });
    }

    pub fn save(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.save_web();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.save_file.is_none() {
                self.choose_new_save_location();
            }
            self.save_native();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&mut self) {
        self.choose_new_save_location();
        self.save_native();
    }

    fn load_simulation_from_string(&mut self, data: &str) -> Result<(), String> {
        let mut app = Self::empty(self.tutorial_page.is_some());

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.save_file = self.save_file.clone();
        }

        let mut numbers = data.split_whitespace();

        // Viewport focus and zoom
        app.viewport_focus = Vec2::new(parse_next(&mut numbers)?, parse_next(&mut numbers)?);
        app.viewport_zoom = parse_next::<f64>(&mut numbers)?;
        if app.viewport_zoom <= 0.0 {
            return Err(format!(
                "viewport zoom value cannot be zero or negative ({})",
                app.viewport_zoom
            ));
        }

        app.simulation.tick_rate = parse_next::<usize>(&mut numbers)?; // Tick rate

        // Planets length
        let planets_len = parse_next::<usize>(&mut numbers)?;
        app.simulation.planets.reserve(planets_len);

        // Planets
        for _ in 0..planets_len {
            let planet = Planet {
                pos: Vec2::new(parse_next(&mut numbers)?, parse_next(&mut numbers)?),
                vel: Vec2::new(parse_next(&mut numbers)?, parse_next(&mut numbers)?),
                mass: parse_next(&mut numbers)?,
                locked: parse_next::<usize>(&mut numbers)? != 0,
                popup_open: false,
            };
            if planet.mass <= 0.0 {
                return Err(format!(
                    "planet mass cannot be zero or negative ({})",
                    planet.mass
                ));
            }

            app.simulation.planets.push(planet.as_rc());
        }

        *self = app; // No errors, set current app state

        Ok(())
    }

    /// Ask the user to choose a file, load a simulation from it and store the origin location in `App.save_file`
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_native(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Load simulation")
            .add_filter("Orbit Simulation", &["sim"])
            .pick_file()
        else {
            return;
        };

        self.save_file = Some(path.clone());

        match std::fs::read_to_string(&path) {
            // If file read successfully, parse its contents and load the simulation
            Ok(data) => {
                self.error_message = self.load_simulation_from_string(&data).err();
            }
            // Otherwise, show an error
            Err(err) => {
                self.error_message = Some(format!("Error reading file: {err}"));
            }
        };
    }

    /// Spawn a task to ask the user to choose a file
    #[cfg(target_arch = "wasm32")]
    pub fn load_web(&mut self) {
        self.load_task = Some(Task::spawn(async {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Load simulation")
                .pick_file()
                .await;

            if let Some(f) = file {
                String::from_utf8(f.read().await).ok()
            } else {
                None // guard if file does not exist.
            }
        }));
    }

    /// If a load task exists, attempt to extract its result, then load the simulation data
    #[cfg(target_arch = "wasm32")]
    pub fn process_load_task(&mut self) {
        // guard if the task doesn't exist or hasn't finished, return early
        let Some(Ok(Some(data))) = self.load_task.as_ref().and_then(|t| t.take_output()) else {
            return;
        };

        self.load_task = None;
        let result = self.load_simulation_from_string(&data);
        if let Err(err) = result {
            rfd::MessageDialog::new()
                .set_title("Error")
                .set_description(err)
                .show();
        }
    }

    pub fn show_saveload_options(&mut self, ui: &mut egui::Ui) {
        // Presets
        ui.menu_button("Load preset", |ui| {
            if ui.button("Solar System").clicked() {
                *self = Self::empty(self.tutorial_page.is_some());
                self.load_simulation_from_string(include_str!(
                    "../../assets/simulations/solar_system.sim"
                ))
                .expect("Built-in preset file should be valid");
            }
            if ui.button("Kepler's Second Law demo").clicked() {
                *self = Self::empty(self.tutorial_page.is_some());
                self.load_simulation_from_string(include_str!(
                    "../../assets/simulations/keplers_test.sim"
                ))
                .expect("Built-in preset file should be valid");
                self.simulation.k2l = crate::app::simulation::K2L::new_some();
            }
            if ui.button("Sun-Earth-Moon").clicked() {
                *self = Self::empty(self.tutorial_page.is_some());
                self.load_simulation_from_string(include_str!(
                    "../../assets/simulations/sun_earth_moon.sim"
                ))
                .expect("Built-in preset file should be valid");
            }
            if ui.button("System with comets").clicked() {
                *self = Self::empty(self.tutorial_page.is_some());
                self.load_simulation_from_string(include_str!(
                    "../../assets/simulations/system_with_comets.sim"
                ))
                .expect("Built-in preset file should be valid");
            }
        });

        if ui.button("Load from file").clicked() {
            #[cfg(target_arch = "wasm32")]
            self.load_web();
            #[cfg(not(target_arch = "wasm32"))]
            self.load_native();
        }

        if ui
            .button("Save simulation")
            .on_hover_text_at_pointer("Save simulation to a local file")
            .clicked()
        {
            self.save();
        }

        #[cfg(not(target_arch = "wasm32"))]
        if ui
            .button("Save simulation as...")
            .on_hover_text_at_pointer("Save simulation to a new local file")
            .clicked()
        {
            self.save_as();
        }
    }
}
