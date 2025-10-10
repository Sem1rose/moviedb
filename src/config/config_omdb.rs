#[derive(Clone, Default)]
pub struct OMDBConfig {
    key: String,
}

impl OMDBConfig {
    // pub fn init(&mut self, creds: &Credentials) {
    //     self.set_key(creds.omdb_key.clone());
    // }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn key_owned(&self) -> String {
        self.key.clone()
    }

    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    pub fn has_key(&self) -> bool {
        self.key != *""
    }
}
