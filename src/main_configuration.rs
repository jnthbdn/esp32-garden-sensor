use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};

use crate::string_error::{StringError, StringEspError};

static IS_NVS_TAKEN: AtomicBool = AtomicBool::new(false);

const PARTITION_NAME: &str = "config";
const NAMESPACE: &str = "config";

const KEY_SSID: &str = "SSID";
const KEY_PASSPHRASE: &str = "PASS";
const KEY_VHIGH: &str = "VHIGH";
const KEY_VLOW: &str = "VLOW";
const KEY_ID: &str = "ID";
const KEY_NAME: &str = "NAME";

pub struct MainConfiguration {
    nvs: EspNvs<NvsCustom>,
}

impl MainConfiguration {
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
        self.store_string(KEY_SSID, MainConfiguration::trunc_string(ssid, 32))
    }

    pub fn get_passphrase(&self) -> String {
        self.read_string(KEY_PASSPHRASE, "")
    }

    pub fn set_passphrase(&mut self, passphrase: &str) -> Result<(), StringEspError> {
        self.store_string(
            KEY_PASSPHRASE,
            MainConfiguration::trunc_string(passphrase, 63),
        )
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

    pub fn get_name(&self) -> String {
        self.read_string(KEY_NAME, "")
    }

    pub fn set_name(&mut self, name: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_NAME, MainConfiguration::trunc_string(name, 32))
    }

    pub fn get_id(&self) -> u32 {
        self.nvs.get_u32(KEY_ID).unwrap_or(Some(0)).unwrap_or(0)
    }

    pub fn set_id(&mut self, value: u32) -> Result<(), StringEspError> {
        self.nvs
            .set_u32(KEY_ID, value)
            .map_err(|e| StringEspError("Failed store ID", e))
    }

    fn store_string(&mut self, key: &str, value: &str) -> Result<(), StringEspError> {
        self.nvs
            .set_str(key, value)
            .map_err(|e| StringEspError("Failed to store string", e))
    }

    fn read_string(&self, key: &str, default: &str) -> String {
        let size = self.nvs.str_len(key).unwrap_or(Some(0)).unwrap_or(0);
        let mut buf = vec![0; size];

        if size == 0 {
            return default.to_string();
        }

        self.nvs
            .get_str(key, &mut buf)
            .unwrap_or(Option::Some(default))
            .unwrap_or(default)
            .to_string()
    }

    fn store_float(&mut self, key: &str, value: f32) -> Result<(), StringEspError> {
        let val = u32::from_ne_bytes(value.to_ne_bytes());
        self.nvs
            .set_u32(key, val)
            .map_err(|e| StringEspError("Failed to store float", e))
    }

    fn read_float(&self, key: &str, default: f32) -> f32 {
        // let mut buf = default.to_ne_bytes();

        // match self.nvs.get_blob(key, &mut buf) {
        //     Ok(_) => f32::from_ne_bytes(buf),
        //     Err(_) => return default,
        // }

        match self.nvs.get_u32(key).unwrap_or(None) {
            Some(value) => f32::from_ne_bytes(value.to_ne_bytes()),
            None => default,
        }
    }

    fn trunc_string(s: &str, max: usize) -> &str {
        match s.char_indices().nth(max) {
            None => s,
            Some((idx, _)) => &s[..idx],
        }
    }
}

impl Drop for MainConfiguration {
    fn drop(&mut self) {
        IS_NVS_TAKEN.store(false, Ordering::Relaxed);
    }
}

// use embedded_storage::{ReadStorage, Storage};
// use esp_storage::FlashStorage;

// const FLASH_ADDRESS: u32 = 0x9000;

// pub struct MainConfiguration {
//     flash: FlashStorage,

//     // The WiFi SSID (max 32 chars.)
//     pub ssid: String,

//     // The WiFi passphrase (max 63 chars.)
//     pub passphrase: String,

//     // The voltage when moisture is low (0%)
//     pub vlow_moisture: f32,

//     // The voltage when moisture is high (100%)
//     pub vhigh_moisture: f32,

//     // The sensor human name (max 32 chars.)
//     pub name: String,

//     // The sensor ID
//     pub id: u32,
// }

// impl MainConfiguration {
//     pub fn new(flash_storage: FlashStorage) -> Self {
//         let mut s = MainConfiguration {
//             flash: flash_storage,
//             ssid: String::with_capacity(32),
//             passphrase: String::with_capacity(63),
//             vlow_moisture: 0.0,
//             vhigh_moisture: 0.0,
//             name: String::with_capacity(32),
//             id: 0,
//         };

//         // s.read_configuration();

//         s
//     }

//     pub fn read_configuration(&mut self) {
//         let mut bytes = [0u8; 139];

//         self.flash.read(FLASH_ADDRESS, &mut bytes).unwrap();

//         self.ssid = String::from_utf8(
//             bytes
//                 .iter()
//                 .take(32)
//                 .filter(|&&x| x != 0x00)
//                 .cloned()
//                 .collect(),
//         )
//         .unwrap_or(String::from(""));

//         self.passphrase = String::from_utf8(
//             bytes
//                 .iter()
//                 .skip(32)
//                 .take(63)
//                 .filter(|&&x| x != 0x00)
//                 .cloned()
//                 .collect(),
//         )
//         .unwrap_or(String::from(""));

//         let high: [u8; 4] = bytes
//             .iter()
//             .skip(95)
//             .take(4)
//             .cloned()
//             .collect::<Vec<_>>()
//             .try_into()
//             .unwrap_or([0u8; 4]);
//         self.vhigh_moisture = f32::from_ne_bytes(high);

//         let low: [u8; 4] = bytes
//             .iter()
//             .skip(99)
//             .take(4)
//             .cloned()
//             .collect::<Vec<_>>()
//             .try_into()
//             .unwrap_or([0u8; 4]);
//         self.vhigh_moisture = f32::from_ne_bytes(low);

//         self.name = String::from_utf8(
//             bytes
//                 .iter()
//                 .skip(103)
//                 .take(32)
//                 .filter(|&&x| x != 0x00)
//                 .cloned()
//                 .collect(),
//         )
//         .unwrap_or(String::from(""));

//         let low: [u8; 4] = bytes
//             .iter()
//             .skip(135)
//             .take(4)
//             .cloned()
//             .collect::<Vec<_>>()
//             .try_into()
//             .unwrap_or([0u8; 4]);
//         self.id = u32::from_ne_bytes(low);

//         // log::info!("Read Bytes: {:?}", bytes);
//     }

//     pub fn write_configuration(&mut self) {
//         let mut bytes = [0u8; 139];

//         Self::copy_bytes_into_array(&mut bytes, self.ssid.as_bytes(), 0, 32);
//         Self::copy_bytes_into_array(&mut bytes, self.passphrase.as_bytes(), 32, 63);
//         Self::copy_bytes_into_array(&mut bytes, &self.vhigh_moisture.to_ne_bytes(), 95, 4);
//         Self::copy_bytes_into_array(&mut bytes, &self.vlow_moisture.to_ne_bytes(), 99, 4);
//         Self::copy_bytes_into_array(&mut bytes, self.name.as_bytes(), 103, 32);
//         Self::copy_bytes_into_array(&mut bytes, &self.id.to_ne_bytes(), 135, 4);

//         self.flash.write(FLASH_ADDRESS, &bytes).unwrap();
//         log::info!("Write Bytes: {:?}", bytes);
//     }

//     fn copy_bytes_into_array(arr: &mut [u8], data: &[u8], offset: usize, len: usize) {
//         for (dst, src) in arr[offset..(offset + len)]
//             .iter_mut()
//             .zip(data.iter().take(len))
//         {
//             *dst = *src;
//         }
//     }
// }
