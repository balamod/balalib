use mlua::prelude::*;
use crate::core::need_update;

use crate::mods::*;

mod mods;
mod core;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn echo(_: &Lua, name: String) -> LuaResult<String> {
    Ok(name)
}


#[mlua::lua_module]
fn balalib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    let balalib_table = lua.create_table()?;
    balalib_table.set("echo", lua.create_function(echo)?)?;
    balalib_table.set("fetch_mods", lua.create_function(|lua, ()| fetch_mods(lua, ()))?)?;
    balalib_table.set("get_local_mods", lua.create_function(|lua, ()| get_local_mods(lua, ()))?)?;
    balalib_table.set("need_update", lua.create_function(|lua, ()| need_update(lua, ()))?)?;
    balalib_table.set("version", VERSION)?;
    exports.set("balalib", balalib_table)?;
    Ok(exports)
}