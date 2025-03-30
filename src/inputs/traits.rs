pub trait input {
    pub fn init() -> Result<(), String>;
    pub fn get_input() -> String;
}
