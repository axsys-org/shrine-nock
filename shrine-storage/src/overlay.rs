


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tray {
    Dish,
    Ewer
}

impl Tray {
    pub fn from_string(tray_str: &str) -> Option<Self> {
        match tray_str {
            "dish" => Some(Self::Dish),
            "ewer" => Some(Self::Ewer),
            _ => None,
        }
    }
}

pub enum Rack {

}