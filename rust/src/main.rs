//! Multiterm on GTK 3. A second launch raises the window that is already open.

mod paths;
mod preferences;
mod session;
mod shell;
mod style;
mod window;

use gtk::gio::ApplicationFlags;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;

use crate::paths::APP_ID;
use crate::shell::write_shell_init;

fn main() {
    glib::set_prgname(Some("multiterm"));
    glib::set_application_name("Multiterm");
    paths::ensure_data();
    write_shell_init();

    let bench = std::env::var("MULTITERM_BENCH").ok().as_deref() == Some("1");
    let app_id = if bench { format!("{APP_ID}.Bench") } else { APP_ID.to_string() };
    let flags = if bench { ApplicationFlags::NON_UNIQUE } else { ApplicationFlags::FLAGS_NONE };
    let app = gtk::Application::builder().application_id(app_id).flags(flags).build();
    let holder: std::rc::Rc<std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<window::Ui>>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    app.connect_activate(move |app| {
        if let Some(ui) = holder.borrow().clone() {
            ui.borrow().window.present();
            return;
        }
        let ui = window::open_window(app, bench);
        ui.borrow().window.show_all();
        ui.borrow().window.present();
        *holder.borrow_mut() = Some(ui.clone());
        let holder = holder.clone();
        ui.borrow().window.connect_delete_event(move |_, _| {
            let holder = holder.clone();
            glib::idle_add_local(move || {
                holder.borrow_mut().take();
                ControlFlow::Break
            });
            glib::Propagation::Proceed
        });
    });
    app.run();
}
