use std::collections::HashMap;
use std::error::Error;
use alfred_core::log::debug;
use home_assistant_rest::Client;

const ENTITY_TYPES: [&str; 4] = ["alarm_control_panel", "light", "media_player", "remote"];

pub async fn get(client: &Client) -> Result<HashMap<String, String>, Box<dyn Error>> {
    const POST_REQUEST: &str = "homeassistant.post_service:";
    let services = client.get_services().await?.iter()
        .map(|service| {
        let vec: Vec<(String, String)> = service.services.iter().map(|(key, service)| {
            (key.clone(), service.description.clone())
        }).collect();
        (service.domain.clone(), vec)
    }).collect::<HashMap<String, Vec<(String, String)>>>();
    let map = client.get_states().await?.iter()
        .map(|state| {
            let entity_id = state.entity_id.clone();
            debug!("{entity_id}");
            let entity_type = (*entity_id.split('.').collect::<Vec<&str>>().first().expect("")).to_string();
            (entity_type, state)
        })
        .filter(|(entity_type, _)| ENTITY_TYPES.contains(&entity_type.as_str()))
        .flat_map(|(entity_type, state)| {
            services.get(&entity_type).expect("").iter()
                .map(|(service, description)| {
                    let entity_id = state.entity_id.clone();
                    (
                        format!("{description} {entity_id}"),
                        format!("{POST_REQUEST} {entity_type} {service} {}", state.entity_id)
                    )
                })
                .collect::<HashMap<String, String>>()
        })
        .collect::<HashMap<String, String>>();
    Ok(map)
}
