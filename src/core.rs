use crate::mods::ModInfo;
use crate::utils::{extract_functions, get_lua_files, minify_lua};
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, Value};
use serde_json::Value as JsonValue;
use std::env;
use std::path::Path;
use std::process::Command;

pub fn need_update(lua: &Lua, _: ()) -> LuaResult<bool> {
    let current_version = lua.load("require('balamod_version')").eval::<String>()?;
    super::updater::need_update(current_version)
}

fn lua_value_to_json_value(value: &Value) -> JsonValue {
    match value {
        Value::Nil => JsonValue::Null,
        Value::Boolean(b) => JsonValue::Bool(*b),
        Value::Integer(i) => JsonValue::Number((*i).into()),
        Value::Number(n) => JsonValue::Number(serde_json::Number::from_f64(*n).unwrap()),
        Value::String(s) => JsonValue::String(s.to_str().unwrap().to_string()),
        Value::Table(table) => table_to_json_value(table),
        _ => JsonValue::Null,
    }
}

fn table_to_json_value(table: &Table) -> JsonValue {
    let mut map = serde_json::Map::new();
    let table_clone = table.clone();
    for pair in table_clone.pairs::<Value, Value>() {
        if let Ok((key, value)) = pair {
            if let Value::String(k) = key {
                map.insert(
                    k.to_str().unwrap().to_string(),
                    lua_value_to_json_value(&value),
                );
            } else if let Value::Integer(k) = key {
                map.insert(k.to_string(), lua_value_to_json_value(&value));
            } else if let Value::Number(k) = key {
                map.insert(k.to_string(), lua_value_to_json_value(&value));
            }
        }
    }
    JsonValue::Object(map)
}

pub fn lua_to_json(table: Value) -> LuaResult<String> {
    let json_value = lua_value_to_json_value(&table);
    let json = serde_json::to_string(&json_value);
    match json {
        Ok(json) => Ok(json),
        Err(e) => Err(mlua::Error::RuntimeError(format!("Error: {}", e))),
    }
}

pub fn json_to_lua(lua: &Lua, json: String) -> LuaResult<Value> {
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| mlua::Error::RuntimeError(format!("Error parsing JSON: {}", e)))?;

    json_value_to_lua_value(lua, value)
}

fn json_value_to_lua_value(lua: &Lua, value: serde_json::Value) -> LuaResult<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(b)),
        serde_json::Value::Number(num) => {
            if let Some(n) = num.as_i64() {
                Ok(Value::Integer(n))
            } else if let Some(n) = num.as_f64() {
                Ok(Value::Number(n))
            } else {
                Err(mlua::Error::RuntimeError("Invalid number".to_string()))
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(&s)?)),
        serde_json::Value::Array(arr) => {
            let tbl = lua.create_table()?;
            for (i, elem) in arr.into_iter().enumerate() {
                let lua_value = json_value_to_lua_value(lua, elem)?;
                tbl.set(i + 1, lua_value)?;
            }
            Ok(Value::Table(tbl))
        }
        serde_json::Value::Object(obj) => {
            let tbl = lua.create_table()?;
            for (key, value) in obj.into_iter() {
                let lua_value = json_value_to_lua_value(lua, value)?;
                tbl.set(key, lua_value)?;
            }
            Ok(Value::Table(tbl))
        }
    }
}

pub fn get_love_dir(lua: &Lua) -> LuaResult<String> {
    lua.load("love.filesystem.getSaveDirectory()")
        .eval::<String>()
}

pub fn is_mod_present(lua: &Lua, mod_info: ModInfo) -> LuaResult<bool> {
    let love_dir = get_love_dir(lua)?;
    let mods_dir = format!("{}/mods", love_dir);
    let mod_dir = format!("{}/{}", mods_dir, mod_info.id);
    if !Path::new(&mod_dir).exists() {
        return Ok(false);
    }

    let manifest_path = format!("{}/manifest.json", mod_dir);
    if !Path::new(&manifest_path).exists() {
        return Ok(false);
    }

    let main_path = format!("{}/main.lua", mod_dir);
    Ok(Path::new(&main_path).exists())
}

#[cfg(target_os = "windows")]
pub fn restart() -> LuaResult<()> {
    let exe_path = env::current_exe()?;
    let args: Vec<String> = env::args().collect();
    Command::new(exe_path).args(&args).spawn()?;
    std::process::exit(0);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn restart() -> LuaResult<()> {
    use std::ffi::OsString;
    use std::os::unix::prelude::CommandExt;
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    Command::new("/proc/self/exe").args(&args).exec();
    Ok(())
}

pub fn setup_injection(lua: &Lua) -> LuaResult<()> {
    let files = get_lua_files();

    if lua
        .load("if game_state then return true else return false end")
        .eval::<bool>()?
    {
        println!("Already injected");
        return Ok(());
    }

    // creating the table, it's tables in a table like file_name/function_name/function_code
    let table = lua.create_table()?;
    for (name, content) in files {
        let functions = extract_functions(content);
        let file_table = lua.create_table()?;
        for (fn_name, code) in functions {
            let mut code = code;
            if code.contains("\n") {
                code = minify_lua(code);
            }
            file_table.set(fn_name.clone(), code)?;
        }
        table.set(name, file_table)?;
    }

    lua.globals().set("game_state", table)?;

    Ok(())
}

pub fn inject(
    lua: &Lua,
    file: String,
    function: String,
    code_to_find: String,
    code_to_insert: String,
) -> LuaResult<()> {
    let code_to_insert = minify_lua(code_to_insert);
    let code_to_find = minify_lua(code_to_find);

    let function_code = lua
        .load(format!("return game_state['{}']['{}']", file, function).as_str())
        .eval::<String>()?;

    if function_code.contains(&code_to_find) {
        let new_code = function_code.replace(&code_to_find, &code_to_insert);
        let file_table = lua
            .load(format!("return game_state['{}']", file).as_str())
            .eval::<Table>()?;
        file_table.set(function.clone(), new_code.clone())?;

        // overwrite the old function in a pcall
        lua.load(format!("pcall(function() {} end)", new_code).as_str())
            .exec()?;

        return Ok(());
    }

    Err(mlua::Error::RuntimeError("Code not found".to_string()))
}

pub fn validate_schema(schema: String, data: String) -> LuaResult<String> {
    Ok(super::utils::validate_schema(schema, data))
}
