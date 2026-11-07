//! Hyrulor — native COSMIC panel applet.
//!
//! The UI is deliberately native Rust/libcosmic.

mod app;
mod socket;
mod ui;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<app::AppModel>(())
}
