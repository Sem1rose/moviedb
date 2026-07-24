pub mod main_screen;

use main_screen::MainScreen;
pub enum Screens {
    MainScreen(MainScreen),
}
