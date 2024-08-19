#[cfg(test)]
mod tests {
    use crate::utils::minify_lua;
    use std::fs;
    use crate::updater::get_latest_cli_version;

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
        assert_eq!(
            minify_lua(lua_file),
            r#"function test() print("Hello World!") a = function() print("Hello World!") end a() end test()"#
        );
    }

    #[test]
    fn test_get_last_cli_version() {
        println!("Latest CLI version: {}", get_latest_cli_version());
    }


    // TODO: Add test for sorted_mods
    // {"id": "test", "load_before": ["foo"], "load_after": ["baz", "qux"]}
    // {"id": "foo", "load_before": [], "load_after": ["baz", "qux"]}
    // {"id": "bar", "load_before": ["baz"], "load_after": []}
    // {"id": "baz", "load_before": ["qux"], "load_after": []}
    // {"id": "qux", "load_before": [], "load_after": []}
    // Expected order: bar, baz, qux, test, foo
}
