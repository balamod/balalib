use std::collections::{HashMap, HashSet};

use crate::core::get_love_dir;
use crate::structs::localmod::LocalMod;
use crate::structs::modinfo::ModInfo;
use crate::utils::validate_schema;
use crate::VERSION;
use mlua::prelude::{LuaError, LuaResult, LuaTable};
use mlua::{Lua, Table};

pub fn download_mod(lua: &Lua, mod_info: ModInfo) -> LuaResult<()> {
    let owner = mod_info.url.split("/").nth(3).unwrap();
    let repo = mod_info.url.split("/").nth(4).unwrap();
    let version = mod_info.version;
    let id = mod_info.id;
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}.tar.gz",
        owner, repo, version, id
    );
    let response = client
        .get(url.clone())
        .send()
        .expect("Failed to get response");
    let body = response.bytes().expect("Failed to get body");
    let love_dir = get_love_dir(lua).expect("Failed to get love dir");
    let mods_dir = format!("{}/mods", love_dir);
    let mod_dir = format!("{}/{}", mods_dir, id);
    std::fs::create_dir_all(&mod_dir)?;
    let tar = body.to_vec();
    unpack_tar(&mod_dir, tar.clone()).expect(format!("Failed to unpack tar: {}", url).as_str());
    Ok(())
}

pub fn unpack_tar(dir: &str, tar: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let tar = std::io::Cursor::new(tar);
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tar));
    archive.unpack(dir)?;
    Ok(())
}

pub fn fetch_mods() -> LuaResult<Vec<ModInfo>> {
    let client = reqwest::blocking::Client::new();
    match client
        .get("https://raw.githubusercontent.com/balamod/balamod/master/new_repos.index")
        .send()
    {
        Ok(response) => match response.text() {
            Ok(text) => {
                let mut mods = Vec::new();
                for line in text.lines() {
                    match get_mods_from_repo(line.to_string()) {
                        Ok(mod_infos) => {
                            for mod_info in mod_infos {
                                mods.push(mod_info);
                            }
                        }
                        Err(e) => {
                            return Err(LuaError::RuntimeError(format!("Error: {}", e)));
                        }
                    }
                }

                println!("Got {} mods:", mods.len());
                Ok(mods)
            }
            Err(e) => Err(LuaError::RuntimeError(format!("Error: {}", e))),
        },
        Err(e) => Err(LuaError::RuntimeError(format!("Error: {}", e))),
    }
}

fn get_mods_from_repo(repo_url: String) -> Result<Vec<ModInfo>, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let response = client.get(repo_url).send()?;
    let mods = response.json::<serde_json::Value>()?;
    let mut mod_infos = Vec::new();
    for mod_info in mods.as_array().unwrap() {
        let url = mod_info["url"].as_str().unwrap();
        let id = mod_info["id"].as_str().unwrap();
        let name = mod_info["name"].as_str().unwrap();
        let description = mod_info["description"].as_array().unwrap();
        let version = mod_info["version"].as_str().unwrap();
        let authors = mod_info["authors"].as_array().unwrap();
        mod_infos.push(ModInfo {
            url: url.to_string(),
            id: id.to_string(),
            name: name.to_string(),
            description: description
                .iter()
                .map(|d| d.as_str().unwrap().to_string())
                .collect(),
            version: version.to_string(),
            authors: authors
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect(),
        });
    }
    Ok(mod_infos)
}

pub fn get_local_mods(lua: &Lua) -> LuaResult<Vec<LocalMod>> {
    let schema = include_bytes!("schema/manifest.schema.json");
    let schema = String::from_utf8(schema.to_vec()).unwrap();

    let love_dir = get_love_dir(lua)?;
    let mods_dir = format!("{}/mods", love_dir);
    let mods = std::fs::read_dir(mods_dir)?;
    let mod_dirs = mods.filter(|entry| entry.as_ref().unwrap().path().is_dir());

    let mut local_mods = Vec::new();

    let balamod_version = lua.load("require 'balamod_version'").eval::<String>()?;

    for mod_dir in mod_dirs {
        let mod_dir: String = mod_dir?.path().display().to_string();
        let manifest_file = format!("{}/manifest.json", mod_dir.clone());
        if !std::path::Path::new(&manifest_file).exists() {
            continue;
        }
        if !std::path::Path::new(&format!("{}/main.lua", mod_dir)).exists() {
            continue;
        }

        let manifest = std::fs::read_to_string(manifest_file)?;
        let validation = validate_schema(schema.clone(), manifest.clone());
        if validation != "valid" {
            println!("Validation error: {}", validation);
            continue;
        }

        let mut manifest: LocalMod = serde_json::from_str(&manifest).unwrap();

        match manifest.clone().balalib_version {
            Some(balalib_version) => match balalib_version.chars().next().unwrap() {
                '>' => {
                    let balalib_version = balalib_version.split(">").nth(1).unwrap();
                    if balalib_version <= VERSION {
                        println!(
                            "Balalib version too low: {} for mod {}",
                            balalib_version, manifest.id
                        );
                        continue;
                    }
                }
                '<' => {
                    let balalib_version = balalib_version.split("<").nth(1).unwrap();
                    if balalib_version >= VERSION {
                        println!(
                            "Balalib version too high: {} for mod {}",
                            balalib_version, manifest.id
                        );
                        continue;
                    }
                }
                '=' => {
                    let balalib_version = balalib_version.split("=").nth(1).unwrap();
                    if balalib_version != VERSION {
                        println!(
                            "Balalib version does not match: {} for mod {}",
                            balalib_version, manifest.id
                        );
                        continue;
                    }
                }
                _ => {}
            },
            None => {}
        }

        match manifest.clone().min_balamod_version {
            Some(min_balamod_version) => {
                if balamod_version < min_balamod_version {
                    println!(
                        "Balalib version too low: {} for mod {}",
                        min_balamod_version, manifest.id
                    );
                    continue;
                }
            }
            None => {}
        }

        match manifest.clone().max_balamod_version {
            Some(max_balamod_version) => {
                if balamod_version > max_balamod_version {
                    println!(
                        "Balalib version too high: {} for mod {}",
                        max_balamod_version, manifest.id
                    );
                    continue;
                }
            }
            None => {}
        }

        let folder_name = mod_dir.split("/").last().unwrap();
        let folder_name = folder_name.split("\\").last().unwrap();

        if manifest.id != folder_name {
            println!(
                "Mod id in manifest.json does not match folder name: {} != {}",
                manifest.id, folder_name
            );
            continue;
        }

        manifest.enabled = !std::path::Path::new(&format!("{}/disable.it", mod_dir)).exists();

        local_mods.push(manifest);
    }

    Ok(local_mods)
}

#[derive(PartialEq)]
enum VisitFlag {
    Temporary,
    Permanent,
}

pub fn sort_mods<'a>(lua: &'a Lua, mods_table: LuaTable<'a>) -> LuaResult<LuaTable<'a>> {
    let mut mods: Vec<LuaTable> = vec![];
    for pair in mods_table.clone().pairs::<String, Table>() {
        let (_, value) = pair?;
        mods.push(value);
    }

    // Initialize graph as a Map<String, Set<String>>
    // where the key is the mod id and the value is a set of mod ids to load before said mod
    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
    for mod_table in mods.iter() {
        let id = mod_table.get::<_, String>("id").unwrap();
        graph.insert(id.clone(), HashSet::new());
    }
    for mod_table in mods.iter() {
        let id = mod_table.get::<_, String>("id").unwrap();
        let load_before = mod_table.get::<_, Vec<String>>("load_before").unwrap();
        for before in load_before {
            graph.get_mut(&id).unwrap().insert(before.clone());
        }
        let load_after = mod_table.get::<_, Vec<String>>("load_after").unwrap();
        for after in load_after {
            graph.get_mut(&after).unwrap().insert(id.clone());
        }
    }
    let mut sorted_mod_ids: Vec<String> = Vec::new();
    let mut visited: HashMap<String, VisitFlag> = HashMap::new();
    fn visit(
        id: String,
        graph: &HashMap<String, HashSet<String>>,
        visited: &mut HashMap<String, VisitFlag>,
        sorted_mod_ids: &mut Vec<String>,
    ) -> bool {
        if visited
            .get(&id)
            .is_some_and(|flag| *flag == VisitFlag::Permanent)
        {
            return true;
        }
        if visited
            .get(&id)
            .is_some_and(|flag| *flag == VisitFlag::Temporary)
        {
            return false;
        }
        visited.insert(id.clone(), VisitFlag::Temporary);
        for before in graph.get(&id).unwrap() {
            if !visit(before.clone(), graph, visited, sorted_mod_ids) {
                return false;
            }
        }
        sorted_mod_ids.push(id.clone());
        visited.insert(id.clone(), VisitFlag::Permanent);
        true
    }
    for id in graph.keys() {
        if !visited.contains_key(id) {
            visit(id.clone(), &graph, &mut visited, &mut sorted_mod_ids);
        }
    }

    let mut sorted_mods: Vec<LuaTable> = Vec::new();
    let mod_count = mods.len();
    for (i, id) in sorted_mod_ids.iter().enumerate() {
        let mod_table = mods
            .iter()
            .find(|mod_table| mod_table.get::<_, String>("id").unwrap() == id.to_owned())
            .unwrap();
        mod_table.set("order", mod_count - i).unwrap();
        sorted_mods.push(mod_table.clone());
    }

    let sorted_mods_table = lua.create_table()?;
    for mod_table in sorted_mods {
        sorted_mods_table.set(mod_table.get::<_, String>("id").unwrap(), mod_table)?;
    }

    Ok(sorted_mods_table)
}
