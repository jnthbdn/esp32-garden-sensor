use percent_encoding::percent_decode_str;

const MAX_PARAM_NB: usize = 8;

#[derive(Debug)]
struct PostDataElement {
    key: String,
    value: String,
}

#[derive(Debug)]
pub struct PostData {
    params: [PostDataElement; MAX_PARAM_NB],
}

impl PostData {
    pub fn from_string(s: String) -> Self {
        let mut postdata = Self::default();

        for (idx, data) in s.split('&').enumerate() {
            if idx >= MAX_PARAM_NB {
                continue;
            }

            let split_data: Vec<&str> = data.split('=').collect();

            postdata.params[idx] = PostDataElement {
                key: percent_decode_str(split_data.get(0).unwrap_or(&"NO_NAME"))
                    .decode_utf8_lossy()
                    .to_string(),
                value: percent_decode_str(split_data.get(1).unwrap_or(&"NO_VALUE"))
                    .decode_utf8_lossy()
                    .to_string(),
            };
        }

        postdata
    }

    pub fn read_value(&self, key: &str) -> Option<String> {
        self.params
            .iter()
            .find(|&x| x.key == key)
            .map(|x| x.value.clone())
    }

    pub fn is_key_exists(&self, key: &str) -> bool {
        self.params.iter().any(|x| x.key == key)
    }
}

impl Default for PostData {
    fn default() -> Self {
        Self {
            params: Default::default(),
        }
    }
}

impl Default for PostDataElement {
    fn default() -> Self {
        Self {
            key: Default::default(),
            value: Default::default(),
        }
    }
}
