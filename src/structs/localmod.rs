use crate::core::{get_love_dir, json_to_lua, lua_to_json};
use crate::download_mod;
use crate::structs::modinfo::ModInfo;
use mlua::prelude::{LuaError, LuaResult, LuaValue};
use mlua::{IntoLua, Lua};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModCommand {
    pub name: String,
    pub lua_path: String,
    pub short_description: String,
    pub usage: String,
}

impl IntoLua<'_> for ModCommand {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("name", self.name)?;
        table.set("lua_path", self.lua_path)?;
        table.set("short_description", self.short_description)?;
        table.set("usage", self.usage)?;
        Ok(LuaValue::Table(table))
    }
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
    pub commands: Option<Vec<ModCommand>>,
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
        match local_mod.commands {
            Some(commands) => {
                let commands: Vec<LuaValue> = commands
                    .into_iter()
                    .map(|command| command.into_lua(lua))
                    .collect::<LuaResult<_>>()?;
                table.set("commands", commands)?;
            }
            None => {
                table.set("commands", lua.create_table()?)?;
            }
        };
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
