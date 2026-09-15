use sonic_rs::Value;

pub fn parse_value(json: &str) -> Value {
    sonic_rs::Deserializer::from_str(json)
        .use_rawnumber()
        .deserialize::<Value>()
        .unwrap()
}
