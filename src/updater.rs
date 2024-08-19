#[cfg(any(target_os = "macos", target_os = "linux"))]
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
    use std::io::Write;

    let url = format!(
        "https://github.com/balamod/balamod/releases/download/{}/balamod-{}-windows.exe",
        cli_ver, cli_ver
    );
    let client = reqwest::blocking::Client::builder()
        .user_agent("balalib")
        .build()
        .unwrap();

    let mut response = client.get(&url).send().unwrap();
    let mut file = std::fs::File::create("balamod.tmp").unwrap();
    std::io::copy(&mut response, &mut file).unwrap();
    std::fs::rename("balamod.tmp", "balamod.exe").unwrap();
    drop(file);

    let script = include_bytes!("scripts/update.cmd");

    let mut file = std::fs::File::create("update.cmd").unwrap();
    file.write_all(script).unwrap();
    drop(file);

    // opens it in a new cmd window
    std::process::Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("update.cmd")
        .spawn()?;

    std::process::exit(0);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn self_update(cli_ver: &str) -> LuaResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut filename = format!("balamod-{}-", cli_ver);
    if cfg!(target_os = "macos") {
        filename.push_str("mac.pkg");
    } else {
        filename.push_str("linux");
    }

    let url = format!("https://github.com/balamod/balamod/releases/download/{}/{}", cli_ver, filename);
    let client = reqwest::blocking::Client::builder()
        .user_agent("balalib")
        .build()
        .unwrap();

    let mut response = client.get(&url).send().unwrap();

    println!("Got response");

    let mut file = std::fs::File::create("balamod.tmp").unwrap();
    println!("Created file");
    std::io::copy(&mut response, &mut file).unwrap();
    println!("Copied response to file");
    std::fs::set_permissions("balamod.tmp", std::fs::Permissions::from_mode(0o755))?;
    println!("Set permissions");

    // Move the temp file to the final name atomically
    std::fs::rename("balamod.tmp", "balamod")?;
    println!("Renamed file");

    // Ensure the previous instance is completely terminated
    std::thread::sleep(std::time::Duration::from_secs(1));

    //close file
    drop(file);

    if cfg!(target_os = "macos") {
        let output = std::process::Command::new("./balamod")
            .arg("-u")
            .arg("-a")
            .output()?;
        println!("{:?}", output);
    } else {
        let output = std::process::Command::new("./balamod")
            .arg("-u")
            .arg("-a")
            .arg("--linux-native")
            .output()?;
        println!("{:?}", output);
    }

    restart()
}

pub fn get_latest_cli_version() -> String {
    let client = reqwest::blocking::Client::builder()
        .user_agent("balalib")
        .build()
        .unwrap();

    match client
        .get("https://api.github.com/repos/balamod/balamod/releases")
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
                latest_version.to_string()
            }
            Err(_) => "0.0.0".to_string(),
        },
        Err(_) => "0.0.0".to_string(),
    }
}