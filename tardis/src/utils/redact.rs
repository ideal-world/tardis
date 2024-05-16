/// Use to mask some sensitive data in the logs
pub trait Redact: Sized {
    fn redact(&self) -> Self;
}

const PASSWORD_MASK: &str = "**";
const STRING_MASK: &str = "[REDACTED]";

impl Redact for url::Url {
    /// Redact the password part of the URL
    fn redact(&self) -> Self {
        let mut url = self.clone();
        if url.password().is_some() {
            let _ = url.set_password(Some(PASSWORD_MASK));
        }
        url
    }
}

impl Redact for String {
    /// Redact the string
    fn redact(&self) -> Self {
        STRING_MASK.to_string()
    }
}
