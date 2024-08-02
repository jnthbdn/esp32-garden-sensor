use super::nvs_configuration::*;
use mutually_exclusive_features::exactly_one_of;

exactly_one_of!("moisture-sensor", "water-level-sensor");

#[derive(Debug)]
pub enum MapFormType {
    String(&'static str, usize),
    Float(f32),
    U32Hex(u32),
    Unsigned64(u64),
    Unsigned8(u8),
}

#[derive(Debug)]
pub struct MapFormElement {
    pub nvs_key: &'static str,
    pub form_name: &'static str,
    pub template_id: Option<&'static str>,
    pub data_type: MapFormType,
}

pub const MAP_NVS_FORM: &[MapFormElement] = &[
    MapFormElement {
        nvs_key: &KEY_SSID,
        form_name: "ssid",
        template_id: Some("{SSID}"),
        data_type: MapFormType::String("", 32),
    },
    MapFormElement {
        nvs_key: &KEY_PASSPHRASE,
        form_name: "pass",
        template_id: Some("{PASS}"),
        data_type: MapFormType::String("", 63),
    },
    MapFormElement {
        nvs_key: &KEY_SERVER_ADDRESS,
        form_name: "srvaddr",
        template_id: Some("{ARVADDR}"),
        data_type: MapFormType::String("", 128),
    },
    MapFormElement {
        nvs_key: &KEY_NAME,
        form_name: "name",
        template_id: Some("{NAME}"),
        data_type: MapFormType::String("", 32),
    },
    MapFormElement {
        nvs_key: &KEY_ID,
        form_name: "id",
        template_id: Some("{ID}"),
        data_type: MapFormType::U32Hex(0),
    },
    MapFormElement {
        nvs_key: &KEY_SLEEP,
        form_name: "sleep",
        template_id: Some("{SLEEP}"),
        data_type: MapFormType::Unsigned64(3600_000_000),
    },
    MapFormElement {
        nvs_key: &KEY_TX_POWER,
        form_name: "txpwr",
        template_id: Some("{TXPWR}"),
        data_type: MapFormType::Unsigned8(80),
    },
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_MOIST_VHIGH,
        form_name: "vhigh_moist",
        template_id: Some("{VHIGH_MOIST}"),
        data_type: MapFormType::Float(1.26),
    },
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_MOIST_VLOW,
        form_name: "vlow_moist",
        template_id: Some("{VLOW_MOIST}"),
        data_type: MapFormType::Float(2.55),
    },
    #[cfg(feature = "water-level-sensor")]
    MapFormElement {
        nvs_key: &KEY_WATER_HIGH,
        form_name: "water_high",
        template_id: Some("{WATER_HIGH}"),
        data_type: MapFormType::Float(20.0),
    },
    #[cfg(feature = "water-level-sensor")]
    MapFormElement {
        nvs_key: &KEY_WATER_LOW,
        form_name: "water_low",
        template_id: Some("{WATER_LOW}"),
        data_type: MapFormType::Float(1020.0),
    },
];

pub fn make_http_url(config: &NvsConfiguration) -> String {
    #[cfg(feature = "moisture-sensor")]
    let endpoint = "send_soil_moisture";

    #[cfg(feature = "water-level-sensor")]
    let endpoint = "send_water_level";

    format!("http://{}/{}", config.get_server_address(), endpoint)
}
