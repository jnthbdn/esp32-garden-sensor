use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};

use crate::string_error::{StringError, StringEspError};

static IS_NVS_TAKEN: AtomicBool = AtomicBool::new(false);

const PARTITION_NAME: &str = "config";
const NAMESPACE: &str = "config";

pub const KEY_SSID: &str = "SSID";
pub const KEY_PASSPHRASE: &str = "PASS";
pub const KEY_ID: &str = "ID";
pub const KEY_NAME: &str = "NAME";
pub const KEY_SLEEP: &str = "SLEEP";

pub const KEY_VHIGH: &str = "VHIGH";
pub const KEY_VLOW: &str = "VLOW";

pub struct NvsConfiguration {
    nvs: EspNvs<NvsCustom>,
}

impl NvsConfiguration {
    pub fn new() -> Result<Self, StringError> {
        if IS_NVS_TAKEN.load(Ordering::Relaxed) {
            return Err(StringError("MainConfiguration NVS already taken"));
        }

        IS_NVS_TAKEN.store(true, Ordering::Relaxed);

        let nvs_custom = match EspCustomNvsPartition::take(PARTITION_NAME) {
            Ok(nvs) => nvs,
            Err(_) => return Err(StringError("Fail to take partition")),
        };

        match EspNvs::new(nvs_custom, NAMESPACE, true) {
            Ok(nvs) => Ok(Self { nvs }),
            Err(_) => Err(StringError("Failed to create EspNvs. Bad namespace ?")),
        }
    }

    pub fn get_ssid(&self) -> String {
        self.read_string(KEY_SSID, "")
    }

    pub fn set_ssid(&mut self, ssid: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_SSID, NvsConfiguration::trunc_string(ssid, 32))
    }

    pub fn get_passphrase(&self) -> String {
        self.read_string(KEY_PASSPHRASE, "")
    }

    pub fn set_passphrase(&mut self, passphrase: &str) -> Result<(), StringEspError> {
        self.store_string(
            KEY_PASSPHRASE,
            NvsConfiguration::trunc_string(passphrase, 63),
        )
    }

    pub fn get_name(&self) -> String {
        self.read_string(KEY_NAME, "")
    }

    pub fn set_name(&mut self, name: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_NAME, NvsConfiguration::trunc_string(name, 32))
    }

    pub fn get_id(&self) -> u32 {
        self.read_unsigned(KEY_ID, 0)
    }

    pub fn set_id(&mut self, value: u32) -> Result<(), StringEspError> {
        self.store_unsigned(KEY_ID, value)
    }

    pub fn get_deep_sleep_duration(&self) -> u64 {
        self.read_unsigned_64(KEY_SLEEP, 3600_000_000)
    }

    pub fn set_deep_sleep_duration(&mut self, value: u64) -> Result<(), StringEspError> {
        self.store_unsigned_64(KEY_SLEEP, value)
    }

    pub fn set_vhigh_moisture(&mut self, value: f32) -> Result<(), StringEspError> {
        self.store_float(KEY_VHIGH, value)
    }

    pub fn get_vhigh_moisture(&self) -> f32 {
        self.read_float(KEY_VHIGH, 0.0)
    }

    pub fn set_vlow_moisture(&mut self, value: f32) -> Result<(), StringEspError> {
        self.store_float(KEY_VLOW, value)
    }

    pub fn get_vlow_moisture(&self) -> f32 {
        self.read_float(KEY_VLOW, 0.0)
    }

    pub fn store_string(&mut self, key: &str, value: &str) -> Result<(), StringEspError> {
        self.remove_existing_key(key)?;
        self.nvs
            .set_str(key, value)
            .map_err(|e| StringEspError("Failed to store string", e))
    }

    pub fn read_string(&self, key: &str, default: &str) -> String {
        let size = self.nvs.str_len(key).unwrap_or(None).unwrap_or(0);
        let mut buf = vec![0; size];

        if size == 0 {
            return default.to_string();
        }

        self.nvs
            .get_str(key, &mut buf)
            .unwrap_or(None)
            .unwrap_or(default)
            .to_string()
    }

    pub fn store_float(&mut self, key: &str, value: f32) -> Result<(), StringEspError> {
        self.remove_existing_key(key)?;
        let val = u32::from_ne_bytes(value.to_ne_bytes());
        self.nvs
            .set_u32(key, val)
            .map_err(|e| StringEspError("Failed to store float", e))
    }

    pub fn read_float(&self, key: &str, default: f32) -> f32 {
        match self.nvs.get_u32(key).unwrap_or(None) {
            Some(value) => f32::from_ne_bytes(value.to_ne_bytes()),
            None => default,
        }
    }

    pub fn store_unsigned(&mut self, key: &str, value: u32) -> Result<(), StringEspError> {
        self.remove_existing_key(key)?;
        self.nvs
            .set_u32(key, value)
            .map_err(|e| StringEspError("Failed to store unsigned", e))
    }

    pub fn read_unsigned(&self, key: &str, default: u32) -> u32 {
        self.nvs.get_u32(key).unwrap_or(None).unwrap_or(default)
    }

    pub fn store_unsigned_64(&mut self, key: &str, value: u64) -> Result<(), StringEspError> {
        self.remove_existing_key(key)?;
        self.nvs
            .set_u64(key, value)
            .map_err(|e| StringEspError("Failed to store u64", e))
    }

    pub fn read_unsigned_64(&self, key: &str, default: u64) -> u64 {
        self.nvs.get_u64(key).unwrap_or(None).unwrap_or(default)
    }

    fn trunc_string(s: &str, max: usize) -> &str {
        match s.char_indices().nth(max) {
            None => s,
            Some((idx, _)) => &s[..idx],
        }
    }

    fn remove_existing_key(&mut self, key: &str) -> Result<(), StringEspError> {
        if self
            .nvs
            .contains(key)
            .map_err(|e| StringEspError("Unable to find key", e))?
        {
            self.nvs
                .remove(key)
                .map_err(|e| StringEspError("Unable to remoive key", e))?;
        }

        Ok(())
    }
}

impl Drop for NvsConfiguration {
    fn drop(&mut self) {
        IS_NVS_TAKEN.store(false, Ordering::Relaxed);
    }
}
