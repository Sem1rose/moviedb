use crate::tokens::Credentials;

#[derive(Clone, Default)]
pub struct OMDBTokens {
    key: String,
}

impl OMDBTokens {
    pub fn new(creds: &Credentials) -> Self {
        Self {
            key: creds.omdb_key.clone(),
        }
    }

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
