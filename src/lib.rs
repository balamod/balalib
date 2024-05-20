use mlua::prelude::*;

fn echo(_: &Lua, name: String) -> LuaResult<String> {
    Ok(name)
}

fn fetch_mods(_: &Lua, _: ()) -> LuaResult<Vec<ModInfo>> {
    let client = reqwest::blocking::Client::new();
    match client.get("https://raw.githubusercontent.com/balamod/balamod/master/new_repos.index").send() {
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
                    return Ok(mods);
                }
                Err(e) => {
                    return Err(LuaError::RuntimeError(format!("Error: {}", e)));
                }
            }
        }
        Err(e) => {
            return Err(LuaError::RuntimeError(format!("Error: {}", e)));
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

#[derive(Debug)]
struct ModInfo {
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


#[mlua::lua_module]
fn balalib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    let balalib_table = lua.create_table()?;
    balalib_table.set("echo", lua.create_function(echo)?)?;
    balalib_table.set("fetch_mods", lua.create_function(|lua, ()| fetch_mods(lua, ()))?)?;
    exports.set("balalib", balalib_table)?;
    Ok(exports)
}