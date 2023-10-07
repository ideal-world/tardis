pub struct Trim;

use super::Mapper;

impl Mapper<String> for Trim {
    type Output = String;
    fn map(value: String) -> String {
        value.trim().to_string()
    }
}

impl<'a> Mapper<&'a str> for Trim {
    type Output = &'a str;
    fn map(value: &'a str) -> &'a str {
        value.trim()
    }
}
