use mlua::{Lua, Table, Value};
use mlua::prelude::LuaResult;
use serde_json::Value as JsonValue;

pub fn need_update(lua: &Lua, _: ()) -> LuaResult<bool> {
    let current_version = lua.load("require('balamod_version')").eval::<String>()?;
    let client = reqwest::blocking::Client::builder().user_agent("balamod_lua").build().unwrap();


    return match client.get("https://api.github.com/repos/balamod/balamod_lua/releases").send() {
        Ok(response) => {
            match response.text() {
                Ok(text) => {
                    let releases: Vec<serde_json::Value> = serde_json::from_str(&text).expect(format!("Failed to parse json: {}", text).as_str());
                    let latest_version = releases.iter().find(|release| {
                        !release["prerelease"].as_bool().unwrap() && !release["draft"].as_bool().unwrap()
                    }).unwrap()["tag_name"].as_str().unwrap();
                    Ok(current_version != latest_version)
                }
                Err(_) => {
                   Ok(false)
                }
            }
        }
        Err(_) => {
            Ok(false)
        }
    }
}

fn lua_value_to_json_value(value: &Value) -> JsonValue {
    match value {
        Value::Nil => JsonValue::Null,
        Value::Boolean(b) => JsonValue::Bool(*b),
        Value::Integer(i) => JsonValue::Number((*i).into()),
        Value::Number(n) => JsonValue::Number(serde_json::Number::from_f64(*n).unwrap()),
        Value::String(s) => JsonValue::String(s.to_str().unwrap().to_string()),
        Value::Table(table) => table_to_json_value(table),
        // Ignorer les fonctions et autres types non pris en charge
        _ => JsonValue::Null,
    }
}

// Fonction pour convertir une table Lua en une valeur JSON
fn table_to_json_value(table: &Table) -> JsonValue {
    let mut map = serde_json::Map::new();
    let table_clone = table.clone();
    for pair in table_clone.pairs::<Value, Value>() {
        if let Ok((key, value)) = pair {
            if let Value::String(k) = key {
                map.insert(k.to_str().unwrap().to_string(), lua_value_to_json_value(&value));
            } else if let Value::Integer(k) = key {
                map.insert(k.to_string(), lua_value_to_json_value(&value));
            } else if let Value::Number(k) = key {
                map.insert(k.to_string(), lua_value_to_json_value(&value));
            }
        }
    }
    JsonValue::Object(map)
}

// Convertit une table Lua en JSON
pub fn lua_to_json(table: Value) -> LuaResult<String> {
    let json_value = lua_value_to_json_value(&table);
    let json = serde_json::to_string(&json_value);
    match json {
        Ok(json) => Ok(json),
        Err(e) => Err(mlua::Error::RuntimeError(format!("Error: {}", e))),
    }
}

pub fn json_to_lua(lua: &Lua, json: String) -> LuaResult<Value> {
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        mlua::Error::RuntimeError(format!("Error parsing JSON: {}", e))
    })?;

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
        },
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(&s)?)),
        serde_json::Value::Array(arr) => {
            let tbl = lua.create_table()?;
            for (i, elem) in arr.into_iter().enumerate() {
                let lua_value = json_value_to_lua_value(lua, elem)?;
                tbl.set(i + 1, lua_value)?;
            }
            Ok(Value::Table(tbl))
        },
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
    lua.load("love.filesystem.getSaveDirectory()").eval::<String>()
}