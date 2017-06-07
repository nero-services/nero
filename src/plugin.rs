pub type LoadFunc = fn() -> bool;
pub type UnloadFunc = fn() -> bool;

#[derive(Debug)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub load: LoadFunc,
    pub unload: UnloadFunc,
}
