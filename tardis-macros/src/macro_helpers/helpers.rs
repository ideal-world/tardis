pub struct ConvertVariableHelpers;

impl ConvertVariableHelpers {
    pub fn underscore_to_camel(s: String) -> String {
        s.split('_').map(|s| s.chars().next().unwrap().to_uppercase().to_string() + &s[1..]).collect()
    }
}
