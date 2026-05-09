use iced::Theme;

use super::CoworkApp;

pub(super) const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);

pub(super) fn title(_: &CoworkApp) -> String {
    String::from("Cowork RS")
}

pub(super) fn theme(_: &CoworkApp) -> Theme {
    Theme::Dark
}
