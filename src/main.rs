use clap::{Parser, Subcommand};
use muzzman_daemon::prelude::*;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    LoadModule {
        name: String,
        index: Option<usize>,
    },
    GetModules {
        index: Option<usize>,
    },
    GetDefaultLocation,
    GetLocation {
        location_id: String,
    },
    #[command(about = "Create a element with the url and try to resolv")]
    Resolv {
        url: String,
        name: Option<String>,
        location: Option<String>,
        #[arg(short, long)]
        progress: bool,
    },
    GetElement {
        element_id: String,
    },
    DestroyElement {
        element_id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let session = DaemonSession::new().expect("Daemon is not started!");
    let session = session.create_session();

    // Check for the daemon
    if let Err(SessionError::ServerTimeOut) = session.get_default_location() {
        eprintln!("Daemon is not started!");
        return;
    }

    let command = cli.command;
    match command {
        Command::LoadModule { name, index } => {
            let mut modules = get_modules();
            modules.retain(|module| module.to_str().unwrap().contains(&name));
            if let Some(index) = index {
                if modules.len() <= index {
                    eprintln!("Invalid intex");
                    return;
                }

                let module = session.load_module(modules[index].clone()).unwrap();
                println!("Loaded module: {}", module.get_name().unwrap());
                println!("Desc: {}", module.get_desc().unwrap());
            } else {
                for (i, module) in modules.iter().enumerate() {
                    println!("{i}: {module:?}");
                }
                eprintln!("Enter the index after the name");
            }
        }
        Command::GetModules { index } => {
            let len = session.get_modules_len().unwrap();
            let modules = session.get_modules(0..len).unwrap();
            if let Some(index) = index {
                if modules.len() <= index {
                    eprintln!("Invaild index");
                    return;
                }
                let module = &modules[index];

                let name = module.get_name().unwrap();
                let default_name = module.get_default_name().unwrap();
                let desc = module.get_desc().unwrap();
                let default_desc = module.get_default_desc().unwrap();
                let proxy = module.get_proxy().unwrap();

                println!("Name: {name}");
                println!("Desc: {desc}");
                println!("DefaultName: {default_name}");
                println!("DefaultDesc: {default_desc}");
                println!("Proxy: {proxy}");
                return;
            }
            for (i, module) in modules.iter().enumerate() {
                let name = module.get_name().unwrap();
                println!("{i}: {name}");
            }
        }
        Command::GetDefaultLocation => {
            let default_location = session.get_default_location().unwrap();
            let id = default_location.id();
            let id = serde_json::to_string(&id).unwrap();
            println!("{id}");
        }
        Command::GetLocation { location_id } => {
            let id: LocationId =
                serde_json::from_str(&location_id).expect("Cannot parse location id");

            let id = session.get_location_ref(&id).unwrap();

            let name = id.get_name().unwrap();
            let desc = id.get_desc().unwrap();
            let path = id.get_path().unwrap();
            let should_save = id.get_should_save().unwrap();
            let len = id.get_locations_len().unwrap();
            let locations_refs = id.get_locations(0..len).unwrap();
            let len = id.get_elements_len().unwrap();
            let elements_refs = id.get_elements(0..len).unwrap();

            let mut locations = Vec::with_capacity(locations_refs.len());
            for _ref in locations_refs {
                locations.push(serde_json::to_string(&_ref.id()).unwrap())
            }

            let mut elements = Vec::with_capacity(elements_refs.len());
            for _ref in elements_refs {
                elements.push(serde_json::to_string(&_ref.id()).unwrap())
            }

            println!("Name: {name}");
            println!("Desc: {desc}");
            println!("Path: {path:?}");
            println!("ShouldSave: {should_save}");
            print!("Locations: {{");
            for location in locations {
                print!("{location}")
            }
            println!("}}");
            print!("Elements: {{");
            for element in elements {
                print!("{element}")
            }
            println!("}}");
        }
        Command::Resolv {
            url,
            name,
            location,
            progress: show_progress,
        } => {
            let location = if let Some(location_id) = location {
                let id: LocationId =
                    serde_json::from_str(&location_id).expect("Invaild location id");
                session.get_location_ref(&id).unwrap()
            } else {
                session.get_default_location().unwrap()
            };

            let name = if let Some(name) = name {
                name
            } else {
                url.split('/').last().unwrap().to_owned()
            };

            let element = location.create_element(&name).unwrap();
            let mut data = Data::default();
            data.add("url", Value::from(Type::String(url.clone())));
            element.set_element_data(data).unwrap();
            if !element.resolv_module().unwrap() {
                // The ERow Cannot be transfered to the network and is never needed!
                let _ = element.destroy();
                eprintln!("Cannot resolv element");
                return;
            }
            element.init().unwrap();
            let mut data = element.get_element_data().unwrap();
            data.set("url", Type::String(url));
            element.set_element_data(data).unwrap();
            element.set_enabled(true, None).unwrap();
            let id = serde_json::to_string(&element.id()).unwrap();
            if show_progress {
                while element.is_enabled().unwrap() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    println!(
                        "Progress: {}, Status: {}",
                        element.get_progress().unwrap(),
                        element.get_status_msg().unwrap()
                    );
                }
            } else {
                println!("{id}");
            }
        }
        Command::GetElement { element_id } => {
            let id: ElementId = serde_json::from_str(&element_id).expect("Invalid element id!");
            let element = session.get_element_ref(&id).unwrap();

            let name = element.get_name().unwrap();
            let desc = element.get_desc().unwrap();
            let meta = element.get_meta().unwrap();
            let element_data = element.get_element_data().unwrap();
            let module_data = element.get_module_data().unwrap();
            let info = element.get_element_info().unwrap();
            let enabled = element.is_enabled().unwrap();
            let data = element.get_data().unwrap();
            let progress = element.get_progress().unwrap();

            let element_data = serde_json::to_string(&element_data).unwrap();
            let module_data = serde_json::to_string(&module_data).unwrap();
            let info = serde_json::to_string(&info).unwrap();
            let data = serde_json::to_string(&data).unwrap();

            println!("Name: {name}");
            println!("Desc: {desc}");
            println!("Meta: {meta}");
            println!("Progress: {progress}");
            println!("Enabled: {enabled}");
            println!("Element Data: {element_data}\n");
            println!("Module Data: {module_data}\n");
            println!("Info: {info}\n");
            println!("Data: {data}");
        }
        Command::DestroyElement { element_id } => {
            let id: ElementId = serde_json::from_str(&element_id).expect("Invalid element id!");
            let element = session.get_element_ref(&id).unwrap();

            println!("Result: {:?}", element.destroy());
        }
    }
}
