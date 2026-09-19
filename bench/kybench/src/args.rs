//! Minimal `--key value` parser: the bench scripts drive kybench, no need for clap.

use std::collections::HashMap;
use std::str::FromStr;

use crate::spout::Res;

pub struct Args(HashMap<String, String>);

impl Args {
    pub fn parse(mut argv: impl Iterator<Item = String>) -> Res<Self> {
        let mut map = HashMap::new();
        while let Some(key) = argv.next() {
            let key = key.strip_prefix("--").ok_or_else(|| format!("expected --key, got '{key}'"))?;
            let value = argv.next().ok_or_else(|| format!("--{key} needs a value"))?;
            map.insert(key.to_string(), value);
        }
        Ok(Self(map))
    }

    pub fn opt(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    pub fn str(&self, key: &str, default: &str) -> String {
        self.opt(key).unwrap_or(default).to_string()
    }

    pub fn num<T: FromStr>(&self, key: &str, default: T) -> Res<T> {
        match self.opt(key) {
            Some(v) => v.parse().map_err(|_| format!("--{key}: invalid value '{v}'").into()),
            None => Ok(default),
        }
    }
}
