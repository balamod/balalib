use crate::download_mod;
use mlua::prelude::{LuaResult, LuaValue};
use mlua::{FromLua, IntoLua, Lua};

#[derive(Debug, Clone)]
pub struct ModInfo {
    pub url: String,
    pub id: String,
    pub name: String,
    pub description: Vec<String>,
    pub version: String,
    pub authors: Vec<String>,
}

impl IntoLua<'_> for ModInfo {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        let download_mod = self.clone();
        let download_func = lua.create_function(move |lua, ()| download_mod.download(lua))?;
        table.set("url", self.url)?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("description", self.description)?;
        table.set("version", self.version)?;
        table.set("authors", self.authors)?;
        table.set("download", download_func)?;
        Ok(LuaValue::Table(table))
    }
}

impl FromLua<'_> for ModInfo {
    fn from_lua(value: LuaValue, _: &'_ Lua) -> LuaResult<Self> {
        let table = value.as_table().expect("Expected table");
        Ok(ModInfo {
            url: table.get("url")?,
            id: table.get("id")?,
            name: table.get("name")?,
            description: table.get("description")?,
            version: table.get("version")?,
            authors: table.get("authors")?,
        })
    }
}

impl ModInfo {
    pub fn download(&self, lua: &Lua) -> LuaResult<()> {
        download_mod(lua, self.clone())
    }
}
