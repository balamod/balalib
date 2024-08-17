#[cfg(test)]
mod tests {
    use std::fs;
    use crate::utils::minify_lua;

    #[test]
    fn test_update() {
        let version = String::from("v0.1.10");
        assert!(crate::updater::need_update(version).unwrap());
    }

    #[test]
    fn test_mods_fetch() {
        let mods = crate::mods::fetch_mods().unwrap();
        assert!(mods.len() > 0);
    }

    #[test]
    fn test_parsing() {
        let lua_file = fs::read_to_string("test.lua").unwrap();
        let functions = crate::utils::extract_functions(lua_file.clone());
        assert_eq!(functions.len(), 1);
        functions.get("test").unwrap();
        assert_eq!(minify_lua(lua_file), r#"function test() print("Hello World!") a = function() print("Hello World!") end a() end test()"#);
    }
}