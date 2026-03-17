mod app;
mod functions;
mod ref_view;
use app::*;
use leptos::prelude::*;
include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));
use crate::i18n::*;
fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|| {
        view! {
            <I18nContextProvider>
                <App />
            </I18nContextProvider>
        }
    })
}
