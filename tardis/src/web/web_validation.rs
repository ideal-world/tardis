use std::fmt::{Display, Formatter};

use poem_openapi::Validator;

use crate::TardisFuns;

pub struct Phone;

pub struct Mail;

impl Display for Phone {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid phone number format")
    }
}

impl Display for Mail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid mail format")
    }
}

impl Validator<String> for Phone {
    fn check(&self, value: &String) -> bool {
        TardisFuns::field.is_phone(value)
    }
}

impl Validator<String> for Mail {
    fn check(&self, value: &String) -> bool {
        TardisFuns::field.is_mail(value)
    }
}
