use serde_json::{Map, Value};

pub trait Sensor {
    fn add_json_value(&mut self, map: &mut Map<String, Value>);
    fn pretty_print(&mut self) -> String;
}
