mod main_screen;

pub use main_screen::MainScreen;
pub enum Screens {
    MainScreen(MainScreen),
}
