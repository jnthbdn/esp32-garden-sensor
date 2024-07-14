use esp_idf_svc::wifi::AccessPointInfo;

use crate::configuration::{main_configuration, nvs_configuration::NvsConfiguration};

const BASE_HTML: &str = include_str!("html/base.html");

#[cfg(feature = "moisture-sensor")]
const SENSOR_FORM_MOISTURE_HTML: &str = include_str!("html/form_moisture.html");

pub fn to_html(
    main_config: &NvsConfiguration,
    error_message: Option<String>,
    aps: Option<Vec<AccessPointInfo>>,
) -> String {
    #[cfg(feature = "moisture-sensor")]
    generate_html(main_config, error_message, aps, SENSOR_FORM_MOISTURE_HTML)
}

fn generate_html(
    main_config: &NvsConfiguration,
    error_message: Option<String>,
    aps: Option<Vec<AccessPointInfo>>,
    form_setting: &str,
) -> String {
    let mut template = BASE_HTML.to_string();

    template = template.replace("{FORM_SETTINGS}", form_setting);
    template = template.replace("{ERROR_MSG}", &error_message.unwrap_or("".to_string()));
    template = template.replace("{AP_LIST}", &accespoint_to_template(aps));

    for elem in main_configuration::MAP_NVS_FORM {
        if elem.template_id.is_none() {
            continue;
        }

        template = match elem.data_type {
            main_configuration::MapFormType::String(default, _) => template.replace(
                elem.template_id.unwrap(),
                &main_config.read_string(&elem.nvs_key, default),
            ),

            main_configuration::MapFormType::Float(default) => template.replace(
                elem.template_id.unwrap(),
                &format!("{}", main_config.read_float(&elem.nvs_key, default)),
            ),

            main_configuration::MapFormType::U32Hex(default) => template.replace(
                elem.template_id.unwrap(),
                &format!("{:x}", main_config.read_u32(&elem.nvs_key, default)),
            ),

            main_configuration::MapFormType::Unsigned64(default) => template.replace(
                elem.template_id.unwrap(),
                &main_config.read_u64(&elem.nvs_key, default).to_string(),
            ),

            main_configuration::MapFormType::Unsigned8(default) => template.replace(
                elem.template_id.unwrap(),
                &main_config.read_u8(&elem.nvs_key, default).to_string(),
            ),
        };
    }

    template
}

fn accespoint_to_template(aps: Option<Vec<AccessPointInfo>>) -> String {
    let mut result = String::new();

    if aps.is_none() {
        return result;
    } else {
        let aps = aps.unwrap();

        result += "[";
        for ap in aps {
            result += &format!("{{ssid:\"{}\",rssi:{}}},", ap.ssid, ap.signal_strength);
        }
        result += "]";
    }

    result
}
