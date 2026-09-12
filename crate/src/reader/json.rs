pub type Value = sonic_rs::Value;

pub fn parse(input: &str) -> Result<Value, String> {
    sonic_rs::from_str::<Value>(input).map_err(|error| error.to_string())
}
