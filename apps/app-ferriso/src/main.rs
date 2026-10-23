#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod accessibility;
mod app;
mod config;
mod gui;
mod i18n;
mod skin3d;
mod update;

use app::{FerrisoApp, Page};
use config::AppConfig;
use gui::FerrisoGui;

fn startup() -> (Page, Option<String>, Option<String>, Option<String>) {
    let mut page = Page::Home;
    let mut query = None;
    let mut instance = None;
    let mut project = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--page=") {
            if let Some(next) = Page::from_arg(value) {
                page = next;
            }
        } else if arg == "--page" {
            if let Some(next) = args.next().as_deref().and_then(Page::from_arg)
            {
                page = next;
            }
        } else if let Some(value) = arg.strip_prefix("--search=") {
            query = Some(value.to_string());
        } else if arg == "--search" {
            query = args.next();
        } else if let Some(value) = arg.strip_prefix("--instance=") {
            instance = Some(value.to_string());
        } else if arg == "--instance" {
            instance = args.next();
        } else if let Some(value) = arg.strip_prefix("--project=") {
            project = Some(value.to_string());
        } else if arg == "--project" {
            project = args.next();
        }
    }
    (page, query, instance, project)
}

fn startup_settings() -> Option<theseus::data::Settings> {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("could not start settings preload runtime: {error}");
            return None;
        }
    };
    match runtime.block_on(theseus::settings::get_startup(app::APP_IDENTIFIER))
    {
        Ok(settings) => Some(settings),
        Err(error) => {
            eprintln!("could not preload settings: {error}");
            None
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match AppConfig::load() {
        Ok(config) => config.sanitized(),
        Err(error) => {
            eprintln!("{}", ferriso_diagnostics::render(&error));
            std::process::exit(2);
        }
    };
    let (page, query, instance, project) = startup();
    theseus::start_logger(app::APP_IDENTIFIER);
    i18n::set_locale(&i18n::system_locale());
    let startup_settings = startup_settings();
    if let Some(settings) = startup_settings.as_ref() {
        i18n::set_locale(&settings.locale);
    }
    let mut app = if let Some(project) = project {
        FerrisoApp::with_project(project)
    } else if let Some(instance) = instance {
        FerrisoApp::with_instance(instance)
    } else if let Some(query) = query {
        FerrisoApp::with_browse_query(query)
    } else {
        FerrisoApp::with_page(page)
    };
    if let Some(settings) = startup_settings {
        app = app.with_initial_settings(settings);
    }
    let app = app.with_update_channel(config.update.channel);

    ferriso_gui::run(
        app,
        FerrisoGui::with_title(config.gui.title.clone()),
        config.gui.settings(),
    )?;
    Ok(())
}
