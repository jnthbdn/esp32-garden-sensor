use super::nvs_configuration::*;

#[cfg(all(feature = "moisture-sensor", feature = "water-level-sensor"))]
compile_error!("Choose only one sensor.");

#[cfg(not(any(feature = "moisture-sensor", feature = "water-level-sensor")))]
compile_error!("Choose one sensor feature.");

#[derive(Debug)]
pub enum MapFormType {
    String(&'static str, usize),
    Float(f32),
    U32Hex(u32),
    Unsigned64(u64),
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
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_VHIGH,
        form_name: "vhigh_moist",
        template_id: Some("{VHIGH_MOIST}"),
        data_type: MapFormType::Float(1.26),
    },
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_VLOW,
        form_name: "vlow_moist",
        template_id: Some("{VLOW_MOIST}"),
        data_type: MapFormType::Float(2.55),
    },
];
