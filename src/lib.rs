#[cfg(not(target_os = "android"))]
use crate::core::restart;
use crate::core::{
    inject, is_mod_present, json_to_lua, lua_to_json, need_update, setup_injection, validate_schema,
};
use mlua::prelude::*;
use mlua::Value;

use crate::mods::*;
#[cfg(not(target_os = "android"))]
use crate::updater::{get_latest_cli_version, self_update};

mod core;
mod mods;
mod tests;
mod updater;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn echo(_: &Lua, name: String) -> LuaResult<String> {
    Ok(name)
}

#[mlua::lua_module]
fn balalib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("echo", lua.create_function(echo)?)?;
    exports.set("fetch_mods", lua.create_function(|_, ()| fetch_mods())?)?;
    exports.set(
        "get_local_mods",
        lua.create_function(|lua, ()| get_local_mods(lua))?,
    )?;
    exports.set(
        "need_update",
        lua.create_function(|lua, ()| need_update(lua, ()))?,
    )?;
    exports.set(
        "download_mod",
        lua.create_function(|lua, mod_info: ModInfo| download_mod(lua, mod_info))?,
    )?;
    exports.set(
        "lua_to_json",
        lua.create_function(|_, table: Value| lua_to_json(table))?,
    )?;
    exports.set(
        "json_to_lua",
        lua.create_function(|lua, json: String| json_to_lua(lua, json))?,
    )?;
    exports.set(
        "is_mod_present",
        lua.create_function(|lua, mod_info: ModInfo| is_mod_present(lua, mod_info))?,
    )?;
    #[cfg(not(target_os = "android"))]
    exports.set(
        "self_update",
        lua.create_function(|_, ()| self_update(get_latest_cli_version().as_str()))?,
    )?;
    #[cfg(not(target_os = "android"))]
    exports.set("restart", lua.create_function(|_, ()| restart())?)?;
    exports.set(
        "setup_injection",
        lua.create_function(|lua, ()| setup_injection(lua))?,
    )?;
    exports.set(
        "validate_schema",
        lua.create_function(|_, (schema, data): (String, String)| validate_schema(schema, data))?,
    )?;
    exports.set("inject", lua.create_function(|lua, (file, function, code_to_find, code_to_insert): (String, String, String, String)| inject(lua, file, function, code_to_find, code_to_insert))?)?;
    exports.set("version", VERSION)?;
    exports.set(
        "sort_mods",
        lua.create_function(|lua, mods: LuaTable| sort_mods(lua, mods))?,
    )?;
    lua.load(format!("G.VERSION = G.VERSION .. '\\nBalalib {}'", VERSION).as_str())
        .exec()?;
    Ok(exports)
}
