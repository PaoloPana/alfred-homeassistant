mod capabilities;

use std::error::Error;
use alfred_core::{AlfredModule, ModuleDetailsBuilder};
use alfred_core::config::Config;
use alfred_core::log::{debug, error};
use alfred_core::message::{Message, MessageType};
use alfred_core::tokio;
use home_assistant_rest::{Client, StateEnum};
use home_assistant_rest::post::CallServiceParams;
use serde_json::json;

const MODULE_NAME: &str = "homeassistant";
const POST_SERVICE_TOPIC: &str = "homeassistant.post_service";
const GET_STATE_TOPIC: &str = "homeassistant.get_state";

fn state_to_string(state: StateEnum) -> String {
    match state {
        StateEnum::Integer(i) => { i.to_string() }
        StateEnum::Decimal(d) => { d.to_string() }
        StateEnum::Boolean(b) => { b.to_string() }
        StateEnum::String(s) => { s }
    }
}

async fn get_state_handler(client: &Client, message: &Message) -> Result<(String, Message), Box<dyn Error>> {
    let entity_id = message.text.clone();
    let state = client.get_states_of_entity(&*entity_id).await
        .map_err(Into::<Box<dyn Error>>::into)?
        .state.ok_or_else(|| format!("An error occurred while fetching the entity ({entity_id})."))?;
    message.reply(state_to_string(state), MessageType::Text).map_err(Into::into)
}

async fn post_service_handler(client: &Client, message: &Message) -> Result<(String, Message), Box<dyn Error>> {
    let split = message.text.split(' ').collect::<Vec<&str>>();
    if split.len() != 3 {
        let err_msg = format!("Wrong format: {}", message.text);
        error!("{}", err_msg);
        return Err(err_msg.into());
    }
    let domain = split[0].to_string();
    let service = split[1].to_string();
    let entity_id = split[2].to_string();

    client.post_service(CallServiceParams {
        domain,
        service: service.clone(),
        service_data: Some(json!({"entity_id": entity_id}))
    }).await
        .map_err(Into::into)
        .and_then(|_| message.reply(service, MessageType::Text).map_err(Into::into))
}

fn get_client(config: &Config) -> Result<Client, Box<dyn Error>> {
    let base_url = config.get_module_value("url").expect("Missing home assistant url");
    let token = config.get_module_value("token").expect("Missing home assistant token");
    Ok(Client::new(&base_url, &token)?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some(MODULE_NAME));
    let client = get_client(&config)?;
    let capabilities = capabilities::get(&client).await?;
    debug!("capabilities: {:#?}", capabilities);

    let module_details = ModuleDetailsBuilder::new()
        .module_name(MODULE_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .config(Some(config))
        .capabilities(capabilities)
        .build();
    let mut module = AlfredModule::new_with_details(module_details)
        .await.expect("An error occurred while fetching the module");
    module.listen(MODULE_NAME).await.expect("An error occurred while listening");
    loop {
        let (topic, message) = module.receive().await.expect("An error occurred while fetching the module");
        let res = match topic.as_str() {
            POST_SERVICE_TOPIC => post_service_handler(&client, &message).await,
            GET_STATE_TOPIC => get_state_handler(&client, &message).await,
            _ => {
                Err("Unknown topic".into())
            }
        };
        if let Ok((response_topic, reply)) = res {
            if let Err(err) = module.send(&response_topic, &reply).await {
                error!("An error occurred while sending the reply: {}", err);
            }
        } else {
            error!("An error occurred: {:?}", res.err());
        }
    }
}
