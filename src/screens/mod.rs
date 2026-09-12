pub mod main_screen;

use main_screen::MainScreen;
pub enum Screen {
    MainScreen(MainScreen),
}
