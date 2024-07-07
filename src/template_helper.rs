use crate::{configuration::MAP_NVS_FORM, main_configuration::MainConfiguration};
use log::info;

const BASE_HTML: &str = include_str!("html/base.html");

#[cfg(feature = "moisture-sensor")]
const SENSOR_FORM_HTML: &str = include_str!("html/form_moisture.html");

pub fn template_moisture(main_config: &MainConfiguration, error_message: Option<String>) -> String {
    let mut template = BASE_HTML.to_string();

    template = template.replace("{FORM_SETTINGS}", SENSOR_FORM_HTML);
    template = template.replace("{ERROR_MSG}", &error_message.unwrap_or("".to_string()));

    for elem in MAP_NVS_FORM {
        if elem.template_id.is_none() {
            continue;
        }

        template = match elem.data_type {
            crate::configuration::MapFormType::String => template.replace(
                elem.template_id.unwrap(),
                &main_config.read_string(&elem.nvs_key, ""),
            ),

            crate::configuration::MapFormType::Float => template.replace(
                elem.template_id.unwrap(),
                &format!("{}", main_config.read_float(&elem.nvs_key, 0.0)),
            ),

            crate::configuration::MapFormType::Unsigned => template.replace(
                elem.template_id.unwrap(),
                &format!("{}", main_config.read_unsigned(&elem.nvs_key, 0)),
            ),
        };
    }

    template
}
