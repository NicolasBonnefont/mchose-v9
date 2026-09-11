//! Applet COSMIC com a bateria do MCHOSE V9 PRO.

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<cosmic_applet_mchose::app::Window>(())
}
