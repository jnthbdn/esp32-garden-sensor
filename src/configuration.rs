use crate::main_configuration::*;

#[cfg(all(feature = "moisture-sensor", feature = "water-level-sensor"))]
compile_error!("Choose only one sensor.");

#[cfg(not(any(feature = "moisture-sensor", feature = "water-level-sensor")))]
compile_error!("Choose one sensor feature.");

#[derive(Debug)]
pub enum MapFormType {
    String,
    Float,
    Unsigned,
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
        data_type: MapFormType::String,
    },
    MapFormElement {
        nvs_key: &KEY_PASSPHRASE,
        form_name: "pass",
        template_id: None,
        data_type: MapFormType::String,
    },
    MapFormElement {
        nvs_key: &KEY_NAME,
        form_name: "name",
        template_id: Some("{NAME}"),
        data_type: MapFormType::String,
    },
    MapFormElement {
        nvs_key: &KEY_NAME,
        form_name: "id",
        template_id: Some("{ID}"),
        data_type: MapFormType::Unsigned,
    },
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_VHIGH,
        form_name: "vhigh_moist",
        template_id: Some("{VHIGH_MOIST}"),
        data_type: MapFormType::Float,
    },
    #[cfg(feature = "moisture-sensor")]
    MapFormElement {
        nvs_key: &KEY_VLOW,
        form_name: "vlow_moist",
        template_id: Some("{VLOW_MOIST}"),
        data_type: MapFormType::Float,
    },
];
