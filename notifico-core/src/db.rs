use std::fs::OpenOptions;
use url::Url;

pub fn create_sqlite_if_not_exists(db_url: &Url) {
    if db_url.scheme() == "sqlite" {
        let url_string = db_url.to_string();
        let file: Vec<&str> = url_string
            .trim_start_matches("sqlite://")
            .split("?")
            .collect();
        let _ = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(file[0]);
    }
}
