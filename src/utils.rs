use regex::Regex;
use std::collections::HashMap;
use std::io::Read;
#[cfg(not(all(target_os = "macos", not(debug_assertions))))]
use std::path::Path;
use std::{env, fs};

pub fn minify_lua(code: String) -> String {
    fn remove_comments(code: &str) -> String {
        let mut in_string = false;
        let mut in_block_comment = false;
        let mut result = String::new();
        let mut iter = code.chars().peekable();

        while let Some(ch) = iter.next() {
            if ch == '"' || ch == '\'' {
                in_string = !in_string;
                result.push(ch);
                continue;
            }

            if !in_string {
                // Handle block comments
                if !in_block_comment && ch == '-' {
                    if let Some('-') = iter.peek() {
                        iter.next();
                        if let Some('[') = iter.peek() {
                            iter.next();
                            if let Some('[') = iter.peek() {
                                iter.next();
                                in_block_comment = true;
                                continue;
                            }
                        } else {
                            for next_char in iter.by_ref() {
                                if next_char == '\n' {
                                    result.push('\n');
                                    break;
                                }
                            }
                            continue;
                        }
                    }
                }

                if in_block_comment && ch == ']' {
                    if let Some(']') = iter.peek() {
                        iter.next();
                        in_block_comment = false;
                        continue;
                    }
                }
            }

            if !in_block_comment {
                result.push(ch);
            }
        }

        result
    }

    fn minimize_whitespace(code: &str) -> String {
        let mut in_string = false;
        let mut last_char_was_whitespace = false;
        let mut result = String::new();

        for ch in code.chars() {
            if ch == '"' || ch == '\'' {
                in_string = !in_string;
            }

            if in_string {
                result.push(ch);
            } else if ch.is_whitespace() {
                if !last_char_was_whitespace {
                    result.push(' ');
                    last_char_was_whitespace = true;
                }
            } else {
                result.push(ch);
                last_char_was_whitespace = false;
            }
        }

        result
    }

    let code_without_comments = remove_comments(&code);

    minimize_whitespace(&code_without_comments)
}

pub fn extract_functions(minified_code: String) -> HashMap<String, String> {
    // Regex to find function definitions
    let re_func = Regex::new(r"(?m)^function\s+([a-zA-Z0-9_:]+)\s*\(.*?\)").unwrap();
    // Regex to find the end of the function
    let re_end = Regex::new(r"(?m)^end\s*$").unwrap();

    let mut functions = HashMap::new();
    let lines: Vec<&str> = minified_code.split('\n').collect();
    let mut inside_function = false;
    let mut function_name = String::new();
    let mut function_code = Vec::new();

    for line in lines {
        if inside_function {
            function_code.push(line);
            if re_end.is_match(line) {
                inside_function = false;
                functions.insert(function_name.clone(), function_code.join("\n"));
                function_code.clear();
            }
        } else if let Some(captures) = re_func.captures(line) {
            function_name = captures.get(1).unwrap().as_str().to_string();
            function_code.push(line);
            inside_function = true;
        }
    }

    functions
}

#[cfg(not(all(target_os = "macos", not(debug_assertions))))]
fn traverse_dir(dir: &Path, prefix: &str, files: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            files.push(format!(
                "{}{}",
                prefix,
                path.file_name().unwrap().to_str().unwrap()
            ));
        } else if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap().to_string();
            traverse_dir(&path, &format!("{}/", dir_name), files);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_lua_files() -> HashMap<String, String> {
    let exe_name = env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    if exe_name == "love" || exe_name == "love.exe" {
        // files are in raw folder (1 arg)
        let path_arg = env::args().nth(1).unwrap();
        let mut map = HashMap::new();
        // only files in the directory
        let mut entries = vec![];
        traverse_dir(Path::new(&path_arg), "", &mut entries);

        for entry in entries {
            if entry.ends_with(".lua") {
                let mut file = fs::File::open(format!("{}/{}", path_arg, entry)).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                map.insert(entry.replace(".lua", ""), content);
            }
        }

        map
    } else {
        let exe_path = env::current_exe().unwrap();
        let file = fs::File::open(exe_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut map = HashMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();
            if !name.ends_with(".lua") {
                continue;
            }
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            map.insert(name, content);
        }
        map
    }
}

#[cfg(all(debug_assertions, target_os = "macos"))]
pub fn get_lua_files() -> HashMap<String, String> {
    // files are in raw folder (1 arg)
    let path_arg = env::args().nth(1).unwrap();
    let mut map = HashMap::new();
    // only files in the directory
    let mut entries = vec![];
    traverse_dir(Path::new(&path_arg), "", &mut entries);

    for entry in entries {
        if entry.ends_with(".lua") {
            let mut file = fs::File::open(format!("{}/{}", path_arg, entry)).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            map.insert(entry.replace(".lua", ""), content);
        }
    }

    map
}

#[cfg(all(not(debug_assertions), target_os = "macos"))]
pub fn get_lua_files() -> HashMap<String, String> {
    let exe_path = env::current_exe().unwrap();
    let mut map = HashMap::new();
    match fs::File::open(exe_path) {
        Ok(file) => {
            match zip::ZipArchive::new(file) {
                Ok(mut archive) => {
                    for i in 0..archive.len() {
                        match archive.by_index(i) {
                            Ok(mut file) => {
                                let name = file.name().to_string();
                                if !name.ends_with(".lua") {
                                    continue;
                                }
                                let mut content = String::new();
                                match file.read_to_string(&mut content) {
                                    Ok(_) => map.insert(name, content),
                                    Err(_) => continue,
                                }
                            }
                            Err(_) => continue,
                        };
                    }
                }
                Err(_) => return HashMap::new(),
            };
        }
        Err(_) => return HashMap::new(),
    };
    return map;
}

pub fn validate_schema(schema: String, data: String) -> String {
    let schema: serde_json::Value = match serde_json::from_str(&schema) {
        Ok(schema) => schema,
        Err(e) => return format!("Error parsing schema: {}", e),
    };
    let data: serde_json::Value = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(e) => return format!("Error parsing data: {}", e),
    };

    let binding = match jsonschema::JSONSchema::compile(&schema) {
        Ok(binding) => binding,
        Err(e) => return format!("Error compiling schema: {}", e),
    };
    let result = binding.validate(&data);

    if result.is_ok() {
        "valid".to_string()
    } else {
        let mut errors = Vec::new();
        for error in result.err().unwrap() {
            errors.push(format!("{}", error));
        }
        errors.join("\n")
    }
}
