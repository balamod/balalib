use mlua::{FromLua, IntoLua, Lua};
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
        let download_func = lua.create_function(|lua, mod_info: ModInfo| download_mod(lua, mod_info))?;
        table.set("download", download_func)?;
        Ok(LuaValue::Table(table))
    }
}

impl FromLua<'_> for ModInfo {
    fn from_lua(value: LuaValue, _: &'_ Lua) -> LuaResult<Self> {
        println!("Type: {}", value.type_name());
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

pub fn download_mod(lua: &Lua, mod_info: ModInfo) -> LuaResult<()> {
    let owner = mod_info.url.split("/").nth(3).unwrap();
    let repo = mod_info.url.split("/").nth(4).unwrap();
    let version = mod_info.version;
    let id = mod_info.id;
    let client = reqwest::blocking::Client::new();
    let url = format!("https://github.com/{}/{}/releases/download/{}/{}.tar.gz", owner, repo, version, id);
    let response = client.get(url.clone()).send().expect("Failed to get response");
    let body = response.bytes().expect("Failed to get body");
    let love_dir = lua.load("love.filesystem.getSaveDirectory()").eval::<String>()?;
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

pub fn fetch_mods(_: &Lua, _: ()) -> LuaResult<Vec<ModInfo>> {
    let client = reqwest::blocking::Client::new();
    return match client.get("https://raw.githubusercontent.com/balamod/balamod/master/new_repos.index").send() {
        Ok(response) => {
            match response.text() {
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
                Err(e) => {
                    Err(LuaError::RuntimeError(format!("Error: {}", e)))
                }
            }
        }
        Err(e) => {
            Err(LuaError::RuntimeError(format!("Error: {}", e)))
        }
    };
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
        let mut manifest: LocalMod = serde_json::from_str(&manifest).unwrap();

        let folder_name = mod_dir.split("/").last().unwrap();
        let folder_name = folder_name.split("\\").last().unwrap();

        if manifest.id != folder_name {
            return Err(LuaError::RuntimeError(format!("Mod id in manifest.json does not match folder name: {} != {}", manifest.id, folder_name)));
        }

        manifest.enabled = !std::path::Path::new(&format!("{}/disable.it", mod_dir)).exists();

        local_mods.push(manifest);
    }

    Ok(local_mods)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalMod {
    pub id: String,
    // default to false
    #[serde(skip)]
    pub enabled: bool,
    pub name: String,
    pub version: String,
    pub description: Vec<String>,
    pub author: String,
    pub load_before: Vec<String>,
    pub load_after: Vec<String>,
}

impl IntoLua<'_> for LocalMod {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("enabled", self.enabled)?;
        table.set("version", self.version)?;
        table.set("description", self.description)?;
        table.set("author", self.author)?;
        table.set("load_before", self.load_before)?;
        table.set("load_after", self.load_after)?;
        Ok(LuaValue::Table(table))
    }
}