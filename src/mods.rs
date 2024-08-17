use crate::core::{get_love_dir, json_to_lua, lua_to_json};
use crate::VERSION;
use jsonschema::JSONSchema;
use mlua::prelude::{LuaError, LuaResult, LuaValue};
use mlua::{FromLua, IntoLua, Lua};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
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
        let download_mod = self.clone();
        let download_func = lua.create_function(move |lua, ()| download_mod.download(lua))?;
        table.set("url", self.url)?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("description", self.description)?;
        table.set("version", self.version)?;
        table.set("authors", self.authors)?;
        table.set("download", download_func)?;
        Ok(LuaValue::Table(table))
    }
}

impl FromLua<'_> for ModInfo {
    fn from_lua(value: LuaValue, _: &'_ Lua) -> LuaResult<Self> {
        let table = value.as_table().expect("Expected table");
        Ok(ModInfo {
            url: table.get("url")?,
            id: table.get("id")?,
            name: table.get("name")?,
            description: table.get("description")?,
            version: table.get("version")?,
            authors: table.get("authors")?,
        })
    }
}

impl ModInfo {
    pub fn download(&self, lua: &Lua) -> LuaResult<()> {
        download_mod(lua, self.clone())
    }
}

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
    let schema: serde_json::Value = serde_json::from_slice(schema).expect("Invalid schema");
    let compiled_schema = JSONSchema::compile(&schema).expect("Invalid schema");

    let love_dir = get_love_dir(lua)?;
    let mods_dir = format!("{}/mods", love_dir);
    let mods = std::fs::read_dir(mods_dir)?;
    let mod_dirs = mods.filter(|entry| entry.as_ref().unwrap().path().is_dir());

    let mut local_mods = Vec::new();

    let balamod_version = lua.load("require 'balamod_version'").eval::<String>()?;

    for mod_dir in mod_dirs {
        let mod_dir: String = mod_dir?.path().display().to_string();
        lua.load(format!(
            "require('logging').getLogger('balalib'):info('Verifying mod {}')",
            mod_dir.clone()
        ))
        .exec()?;
        let manifest_file = format!("{}/manifest.json", mod_dir.clone());
        if !std::path::Path::new(&manifest_file).exists() {
            continue;
        }
        if !std::path::Path::new(&format!("{}/main.lua", mod_dir)).exists() {
            continue;
        }

        let manifest = std::fs::read_to_string(manifest_file)?;
        let manifest_json: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        let validation = compiled_schema.validate(&manifest_json);
        if let Err(errors) = validation {
            for error in errors {
                println!("Validation error: {}", error);
                //lua.load(format!("require('logging').getLogger('balalib'):error('Validation error: {}')", error)).exec()?;
                println!("Instance path: {}", error.instance_path);
                //lua.load(format!("require('logging').getLogger('balalib'):error('Instance path: {}')", error.instance_path)).exec()?;
            }
            //return Err(LuaError::RuntimeError(format!("Invalid manifest.json for mod: {}", mod_dir)));
            continue;
        }

        let mut manifest: LocalMod = serde_json::from_str(&manifest).unwrap();

        match manifest.clone().balalib_version {
            Some(balalib_version) => match balalib_version.chars().next().unwrap() {
                '>' => {
                    let balalib_version = balalib_version.split(">").nth(1).unwrap();
                    if balalib_version <= VERSION {
                        lua.load(format!("require('logging').getLogger('balalib'):error('Balalib version too low: {} for mod {}')", balalib_version, manifest.id)).exec()?;
                        continue;
                    }
                }
                '<' => {
                    let balalib_version = balalib_version.split("<").nth(1).unwrap();
                    if balalib_version >= VERSION {
                        lua.load(format!("require('logging').getLogger('balalib'):error('Balalib version too high: {} for mod {}')", balalib_version, manifest.id)).exec()?;
                        continue;
                    }
                }
                '=' => {
                    let balalib_version = balalib_version.split("=").nth(1).unwrap();
                    if balalib_version != VERSION {
                        lua.load(format!("require('logging').getLogger('balalib'):error('Balalib version does not match: {} for mod {}')", balalib_version, manifest.id)).exec()?;
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
                    lua.load(format!("require('logging').getLogger('balalib'):error('Balalib version too low: {} for mod {}')", min_balamod_version, manifest.id)).exec()?;
                    continue;
                }
            }
            None => {}
        }

        match manifest.clone().max_balamod_version {
            Some(max_balamod_version) => {
                if balamod_version > max_balamod_version {
                    lua.load(format!("require('logging').getLogger('balalib'):error('Balalib version too high: {} for mod {}')", max_balamod_version, manifest.id)).exec()?;
                    continue;
                }
            }
            None => {}
        }

        let folder_name = mod_dir.split("/").last().unwrap();
        let folder_name = folder_name.split("\\").last().unwrap();

        if manifest.id != folder_name {
            lua.load(format!("require('logging').getLogger('balalib'):error('Mod id in manifest.json does not match folder name: {} != {}')", manifest.id, folder_name)).exec()?;
            continue;
        }

        manifest.enabled = !std::path::Path::new(&format!("{}/disable.it", mod_dir)).exists();

        local_mods.push(manifest);
    }

    Ok(local_mods)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalMod {
    pub id: String,
    #[serde(skip)]
    pub enabled: bool,
    pub name: String,
    pub version: String,
    pub description: Vec<String>,
    pub author: String,
    pub load_before: Vec<String>,
    pub load_after: Vec<String>,
    pub min_balamod_version: Option<String>,
    pub max_balamod_version: Option<String>,
    pub balalib_version: Option<String>,
}

impl IntoLua<'_> for LocalMod {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let local_mod = self.clone();
        let table = lua.create_table()?;
        let delete_mod = local_mod.clone();
        let update_mod = local_mod.clone();
        let save_config = local_mod.clone();
        let load_config = local_mod.clone();
        table.set(
            "update",
            lua.create_function(move |lua, mods: Vec<ModInfo>| update_mod.update(lua, mods))?,
        )?;
        table.set(
            "delete",
            lua.create_function(move |lua, ()| delete_mod.delete(lua))?,
        )?;
        table.set(
            "save_config",
            lua.create_function(move |lua, table: LuaValue| save_config.save_config(lua, table))?,
        )?;
        table.set(
            "load_config",
            lua.create_function(move |lua, ()| load_config.load_config(lua))?,
        )?;
        table.set("id", local_mod.id)?;
        table.set("name", local_mod.name)?;
        table.set("enabled", local_mod.enabled)?;
        table.set("version", local_mod.version)?;
        table.set("description", local_mod.description)?;
        table.set("author", local_mod.author)?;
        table.set("load_before", local_mod.load_before)?;
        table.set("load_after", local_mod.load_after)?;

        Ok(LuaValue::Table(table))
    }
}

impl LocalMod {
    pub fn delete(&self, lua: &Lua) -> LuaResult<()> {
        let love_dir = get_love_dir(lua)?;
        let mods_dir = format!("{}/mods", love_dir);
        let mod_dir = format!("{}/{}", mods_dir, self.id);
        std::fs::remove_dir_all(mod_dir)?;
        Ok(())
    }

    pub fn update(&self, lua: &Lua, mods: Vec<ModInfo>) -> LuaResult<()> {
        let mod_info = mods.iter().find(|mod_info| mod_info.id == self.id);
        match mod_info {
            Some(mod_info) => {
                download_mod(lua, mod_info.clone())?;
                println!("Updated mod: {}", self.id);
                Ok(())
            }
            None => {
                println!("Mod not found in the repo: {}", self.id);
                Err(LuaError::RuntimeError(format!(
                    "Mod not found in the repo: {}",
                    self.id
                )))
            }
        }
    }

    pub fn save_config(&self, lua: &Lua, table: LuaValue) -> LuaResult<()> {
        let json = lua_to_json(table)?;
        let love_dir = get_love_dir(lua)?;
        let mods_dir = format!("{}/mods", love_dir);
        let mod_dir = format!("{}/{}", mods_dir, self.id);
        let config_file = format!("{}/config.json", mod_dir);
        std::fs::write(config_file, json)?;
        Ok(())
    }

    pub fn load_config<'lua>(&self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let love_dir = get_love_dir(lua)?;
        let mods_dir = format!("{}/mods", love_dir);
        let mod_dir = format!("{}/{}", mods_dir, self.id);
        let config_file = format!("{}/config.json", mod_dir);
        if !std::path::Path::new(&config_file).exists() {
            println!("No config file found for mod: {}", self.id);
            return Ok(LuaValue::Nil);
        }

        let json = std::fs::read_to_string(config_file)?;
        json_to_lua(lua, json)
    }
}
