use mlua::Lua;
use mlua::prelude::LuaResult;

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