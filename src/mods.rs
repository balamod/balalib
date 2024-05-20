use mlua::{IntoLua, Lua};
use mlua::prelude::{LuaError, LuaResult, LuaValue};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ModInfo {
    pub url: String,
    pub id: String,
    pub name: String,
    pub description: Vec<String>,
    pub version: String,
    pub authors: Vec<String>,
}

impl IntoLua<'_> for ModInfo {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("url", self.url)?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("description", self.description)?;
        table.set("version", self.version)?;
        table.set("authors", self.authors)?;
        Ok(LuaValue::Table(table))
    }
}

pub fn fetch_mods(_: &Lua, _: ()) -> LuaResult<Vec<ModInfo>> {
    let client = reqwest::blocking::Client::new();
    return match client.get("https://raw.githubusercontent.com/balamod/balamod/master/new_repos.index").send() {
        Ok(response) => {
            match response.text() {
                Ok(text) => {
                    let mut mods = Vec::new();
                    for line in text.lines() {
                        println!("Fetching mods from repo: {}", line);
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
                Err(e) => {
                    Err(LuaError::RuntimeError(format!("Error: {}", e)))
                }
            }
        }
        Err(e) => {
            Err(LuaError::RuntimeError(format!("Error: {}", e)))
        }
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
            description: description.iter().map(|d| d.as_str().unwrap().to_string()).collect(),
            version: version.to_string(),
            authors: authors.iter().map(|a| a.as_str().unwrap().to_string()).collect(),
        });
    }
    Ok(mod_infos)
}

pub fn get_local_mods(lua: &Lua, _: ()) -> LuaResult<Vec<LocalMod>> {
    let love_dir = lua.load("love.filesystem.getSaveDirectory()").eval::<String>()?;
    let mods_dir = format!("{}/mods", love_dir);
    let mods = std::fs::read_dir(mods_dir)?;
    let mod_dirs = mods.filter(|entry| entry.as_ref().unwrap().path().is_dir());

    let mut local_mods = Vec::new();
    for mod_dir in mod_dirs {
        let mod_dir: String = mod_dir.unwrap().path().display().to_string();
        let manifest_file = format!("{}/manifest.json", mod_dir.clone());
        if !std::path::Path::new(&manifest_file).exists() {
            continue;
        }
        if !std::path::Path::new(&format!("{}/main.lua", mod_dir)).exists() {
            continue;
        }

        let manifest = std::fs::read_to_string(manifest_file)?;
        let manifest: LocalMod = serde_json::from_str(&manifest).unwrap();

        local_mods.push(manifest);
    }

    Ok(local_mods)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Vec<String>,
    pub author: String,
    pub load_before: Vec<String>,
    pub load_after: Vec<String>
}

impl IntoLua<'_> for LocalMod {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("version", self.version)?;
        table.set("description", self.description)?;
        table.set("author", self.author)?;
        table.set("load_before", self.load_before)?;
        table.set("load_after", self.load_after)?;
        Ok(LuaValue::Table(table))
    }
}