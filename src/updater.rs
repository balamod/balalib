use crate::core::restart;
use mlua::prelude::LuaResult;

pub fn need_update(current_version: String) -> LuaResult<bool> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("balamod_lua")
        .build()
        .unwrap();

    match client
        .get("https://api.github.com/repos/balamod/balamod_lua/releases")
        .send()
    {
        Ok(response) => match response.text() {
            Ok(text) => {
                let releases: Vec<serde_json::Value> = serde_json::from_str(&text)
                    .expect(format!("Failed to parse json: {}", text).as_str());
                let latest_version = releases
                    .iter()
                    .find(|release| {
                        !release["prerelease"].as_bool().unwrap()
                            && !release["draft"].as_bool().unwrap()
                    })
                    .unwrap()["tag_name"]
                    .as_str()
                    .unwrap();
                Ok(current_version != latest_version)
            }
            Err(_) => Ok(false),
        },
        Err(_) => Ok(false),
    }
}

#[cfg(target_os = "windows")]
pub fn self_update(cli_ver: &str) -> LuaResult<()> {
    let url = format!(
        "https://github.com/balamod/balamod/releases/download/{}/balamod-{}-windows.exe",
        cli_ver, cli_ver
    );
    let client = reqwest::blocking::Client::builder()
        .user_agent("balalib")
        .build()
        .unwrap();
    let mut response = client.get(&url).send().unwrap();
    let mut file = std::fs::File::create("balamod.exe").unwrap();
    std::io::copy(&mut response, &mut file).unwrap();
    restart()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn self_update(_cli_ver: &str) -> LuaResult<()> {
    restart()
}
