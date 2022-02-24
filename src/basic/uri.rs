use crate::basic::result::TardisResult;

pub struct TardisUri;

impl TardisUri {
    pub fn format_with_item(&self, host: &str, path_and_query: &str) -> TardisResult<String> {
        if path_and_query.is_empty() {
            self.format(host)
        } else if path_and_query.starts_with('/') && !host.ends_with('/') || !path_and_query.starts_with('/') && host.ends_with('/') {
            self.format(format!("{}{}", host, path_and_query).as_str())
        } else if path_and_query.starts_with('/') && host.ends_with('/') {
            self.format(format!("{}/{}", host, path_and_query).as_str())
        } else {
            self.format(format!("{}/{}", host, &path_and_query[1..]).as_str())
        }
    }

    pub fn format(&self, uri_str: &str) -> TardisResult<String> {
        let uri = url::Url::parse(uri_str)?;
        let host = match uri.host() {
            Some(host) => host,
            None =>
            // E.g. jdbc:h2:men:iam 不用解析
            {
                return Ok(uri.to_string())
            }
        };
        let port = match uri.port() {
            Some(port) => format!(":{}", port),
            None => "".to_string(),
        };
        let path = if uri.path().is_empty() {
            ""
        } else if uri.path().ends_with('/') {
            &uri.path()[..uri.path().len() - 1]
        } else {
            uri.path()
        };
        let query = self.sort_query(uri.query());
        let query = match uri.query() {
            Some(_) => format!("?{}", query),
            None => "".to_string(),
        };
        let formatted_uri = format!("{}://{}{}{}{}", uri.scheme(), host, port, path, query);
        Ok(formatted_uri)
    }

    pub fn get_path_and_query(&self, uri_str: &str) -> TardisResult<String> {
        let uri = url::Url::parse(uri_str)?;
        let path = if uri.path().is_empty() {
            ""
        } else if uri.path().ends_with('/') {
            &uri.path()[..uri.path().len() - 1]
        } else {
            uri.path()
        };
        let query = match uri.query() {
            None => "".to_string(),
            Some(q) => format!("?{}", q),
        };
        return Ok(format!("{}{}", path, query));
    }

    fn sort_query(&self, query: Option<&str>) -> String {
        match query {
            None => "".to_string(),
            Some(query) => {
                let mut query = query.split('&').collect::<Vec<&str>>();
                query.sort_by(|a, b| Ord::cmp(a.split('=').next().unwrap_or(""), b.split('=').next().unwrap_or("")));
                query.join("&")
            }
        }
    }
}
