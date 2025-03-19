pub mod init_screen;
pub mod main_screen;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Screens {
    #[default]
    InitScreen,
    MainScreen,
    TermSizeWarn,
}
