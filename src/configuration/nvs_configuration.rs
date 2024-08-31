use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};
use pad::{Alignment, PadStr};

use crate::string_error::{StringError, StringEspError};

static IS_NVS_TAKEN: AtomicBool = AtomicBool::new(false);

const PARTITION_NAME: &str = "config";
const NAMESPACE: &str = "config";

const PAD_CHAR: char = 0x03 as char;

pub const KEY_SSID: &str = "SSID";
pub const KEY_PASSPHRASE: &str = "PASS";
pub const KEY_SERVER_ADDRESS: &str = "SRVADDR";
pub const KEY_ID: &str = "ID";
pub const KEY_NAME: &str = "NAME";
pub const KEY_SLEEP: &str = "SLEEP";
pub const KEY_TX_POWER: &str = "TX_POWER";

pub const KEY_MOIST_VHIGH: &str = "MVHIGH";
pub const KEY_MOIST_VLOW: &str = "MVLOW";

pub const KEY_WATER_HIGH: &str = "WATERHIGH";
pub const KEY_WATER_LOW: &str = "WATERLOW";

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

    pub fn get_passphrase(&self) -> String {
        self.read_string(KEY_PASSPHRASE, "")
    }

    pub fn get_name(&self) -> String {
        self.read_string(KEY_NAME, "")
    }

    pub fn get_id(&self) -> u32 {
        self.read_u32(KEY_ID, 0)
    }

    pub fn get_server_address(&self) -> String {
        self.read_string(KEY_SERVER_ADDRESS, "192.168.70.1")
    }

    pub fn get_deep_sleep_duration(&self) -> u64 {
        self.read_u64(KEY_SLEEP, 3600_000_000)
    }

    pub fn get_tx_power(&self) -> i8 {
        self.read_u8(KEY_TX_POWER, 80) as i8
    }

    pub fn get_vhigh_moisture(&self) -> f32 {
        self.read_float(KEY_MOIST_VHIGH, 0.0)
    }

    pub fn get_vlow_moisture(&self) -> f32 {
        self.read_float(KEY_MOIST_VLOW, 0.0)
    }

    pub fn get_high_water_level(&self) -> f32 {
        self.read_float(KEY_WATER_HIGH, 0.0)
    }

    pub fn get_low_water_level(&self) -> f32 {
        self.read_float(KEY_WATER_LOW, 0.0)
    }

    pub fn store_string(
        &mut self,
        key: &str,
        value: &str,
        max_size: usize,
    ) -> Result<(), StringEspError> {
        self.nvs
            .remove(key)
            .map_err(|e| StringEspError("Failed to erase key", e))?;
        self.nvs
            .set_str(key, &Self::trunc_pad_string(value, max_size))
            .map_err(|e| StringEspError("Failed to store string", e))
    }

    pub fn read_string(&self, key: &str, default: &str) -> String {
        let size = self.nvs.str_len(key).unwrap_or(None).unwrap_or(0);
        let mut buf = vec![0; size];

        if size == 0 {
            return default.to_string();
        }

        let result = self
            .nvs
            .get_str(key, &mut buf)
            .unwrap_or(None)
            .unwrap_or(default)
            .to_string();

        result
            .split_once(PAD_CHAR)
            .unwrap_or((&result, ""))
            .0
            .to_owned()
    }

    pub fn store_float(&mut self, key: &str, value: f32) -> Result<(), StringEspError> {
        let val = u32::from_ne_bytes(value.to_ne_bytes());
        self.nvs
            .remove(key)
            .map_err(|e| StringEspError("Failed to erase key", e))?;
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

    pub fn store_u8(&mut self, key: &str, value: u8) -> Result<(), StringEspError> {
        self.nvs
            .remove(key)
            .map_err(|e| StringEspError("Failed to erase key", e))?;
        self.nvs
            .set_u8(key, value)
            .map_err(|e| StringEspError("Failed to store U8", e))
    }

    pub fn read_u8(&self, key: &str, default: u8) -> u8 {
        self.nvs.get_u8(key).unwrap_or(None).unwrap_or(default)
    }

    pub fn store_u32(&mut self, key: &str, value: u32) -> Result<(), StringEspError> {
        self.nvs
            .remove(key)
            .map_err(|e| StringEspError("Failed to erase key", e))?;
        self.nvs
            .set_u32(key, value)
            .map_err(|e| StringEspError("Failed to store U32", e))
    }

    pub fn read_u32(&self, key: &str, default: u32) -> u32 {
        self.nvs.get_u32(key).unwrap_or(None).unwrap_or(default)
    }

    pub fn store_u64(&mut self, key: &str, value: u64) -> Result<(), StringEspError> {
        self.nvs
            .remove(key)
            .map_err(|e| StringEspError("Failed to erase key", e))?;
        self.nvs
            .set_u64(key, value)
            .map_err(|e| StringEspError("Failed to store U64", e))
    }

    pub fn read_u64(&self, key: &str, default: u64) -> u64 {
        self.nvs.get_u64(key).unwrap_or(None).unwrap_or(default)
    }

    fn trunc_pad_string(s: &str, max: usize) -> String {
        s.pad(max, PAD_CHAR, Alignment::Left, true)
    }
}

impl Drop for NvsConfiguration {
    fn drop(&mut self) {
        IS_NVS_TAKEN.store(false, Ordering::Relaxed);
    }
}
