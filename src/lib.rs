use crate::core::{
    inject, is_mod_present, json_to_lua, lua_to_json, need_update, restart, self_update,
    setup_injection,
};
use mlua::prelude::*;
use mlua::Value;

use crate::mods::*;

mod core;
mod mods;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn echo(_: &Lua, name: String) -> LuaResult<String> {
    Ok(name)
}

#[mlua::lua_module]
fn balalib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("echo", lua.create_function(echo)?)?;
    exports.set(
        "fetch_mods",
        lua.create_function(|lua, ()| fetch_mods(lua, ()))?,
    )?;
    exports.set(
        "get_local_mods",
        lua.create_function(|lua, ()| get_local_mods(lua, ()))?,
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
    exports.set(
        "self_update",
        lua.create_function(|_, ()| self_update("v0.1.11"))?,
    )?;
    exports.set("restart", lua.create_function(|_, ()| restart())?)?;
    exports.set(
        "setup_injection",
        lua.create_function(|lua, ()| setup_injection(lua))?,
    )?;
    exports.set("inject", lua.create_function(|lua, (file, function, code_to_find, code_to_insert): (String, String, String, String)| inject(lua, file, function, code_to_find, code_to_insert))?)?;
    exports.set("version", VERSION)?;
    lua.load(format!("G.VERSION = G.VERSION .. '\\nBalalib {}'", VERSION).as_str())
        .exec()?;
    Ok(exports)
}
