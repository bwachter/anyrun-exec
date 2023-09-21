use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use glob::glob;
use serde::Deserialize;
use std::env;
use std::{fs, process::Command};

#[derive(Deserialize)]
pub struct Config {
    path: Option<Vec<String>>,
    max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: Option::None,
            max_entries: 5,
        }
    }
}

pub struct State {
    config: Config,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = match fs::read_to_string(format!("{}/exec.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("Error parsing exec plugin config: {}", why);
            Config::default()
        }),
        Err(why) => {
            eprintln!("Error reading exec plugin config: {}", why);
            Config::default()
        }
    };

    State { config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "exec".into(),
        icon: "application-x-executable".into(),
    }
}

fn collect_matches(path_vec: &Vec<String>, command: String, arguments: String) -> RVec<Match> {
    let mut matches: RVec<Match> = RVec::new();

    for path in path_vec.iter() {
        let pattern = format!("{}/{}*", path, command);

        /*
        This lookup could be optimised by keeping a cache of top level matches,
        with narrowing performed on the cache. Performance seems to be good enough,
        though, so probably not worth the effort.
        */
        for entry in glob(pattern.as_str()).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let filename = path.file_name().unwrap();
                    let dirname = path.parent();
                    matches.push(Match {
                        title: format!("{}{}", filename.to_str().unwrap(), arguments).into(),
                        icon: ROption::RNone,
                        description: ROption::RSome(dirname.unwrap().to_str().unwrap().into()),
                        id: ROption::RNone,
                        use_pango: false,
                    })
                }
                Err(e) => println!("{:?}", e),
            }
        }
    }

    matches
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let mut matches: RVec<Match> = RVec::new();
    let command: String;
    let arguments: String;

    if input.contains(" ") {
        let split: Vec<&str> = input.splitn(2, ' ').collect();
        command = split[0].to_string();
        arguments = format!(" {}", split[1].to_string());
    } else {
        command = input.to_string();
        arguments = "".to_string();
    }

    if input.len() >= 1 {
        //let config_path = state.config.path.unwrap();
        //for path in state.config.path.iter() {
        //for path in Some(state.config.path).iter() {
        match &state.config.path {
            Some(path_vec) => {
                matches = collect_matches(path_vec, command, arguments);
            }
            None => {
                if let Ok(env_path) = env::var("PATH") {
                    let mut path_vec: Vec<String> =
                        env_path.split(':').map(str::to_string).collect();
                    path_vec.sort_unstable();
                    path_vec.dedup();
                    matches = collect_matches(&path_vec, command, arguments);
                } else {
                    eprintln!("Empty path, exec plugin will not work.");
                }
            }
        }
    }
    matches.truncate(state.config.max_entries);
    matches
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    if let Err(why) = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{}/{}",
            selection.description.unwrap().to_string(),
            selection.title
        ))
        .current_dir(&env::current_dir().unwrap())
        .spawn()
    {
        eprintln!("Error running executable: {}", why);
    }

    HandleResult::Close
}
