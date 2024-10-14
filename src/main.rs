use std::error::Error;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::interface_module::InterfaceModule;
use alfred_rs::log::{error, warn};
use alfred_rs::message::{Message, MessageType};
use alfred_rs::tokio;
use home_assistant_rest::{Client, StateEnum};
use home_assistant_rest::post::StateParams;

const MODULE_NAME: &'static str = "homeassistant";
const SET_STATE_TOPIC: &'static str = "homeassistant.set_state";
const GET_STATE_TOPIC: &'static str = "homeassistant.get_state";

fn state_to_string(state: StateEnum) -> String {
    match state {
        StateEnum::Integer(i) => { i.to_string() }
        StateEnum::Decimal(d) => { d.to_string() }
        StateEnum::Boolean(b) => { b.to_string() }
        StateEnum::String(s) => { s.clone() }
    }
}

async fn get_state_handler(client: &Client, message: &Message) -> Result<(String, Message), Box<dyn Error>> {
    let entity_id = message.text.clone();
    let entity_response = client.get_states_of_entity(&*entity_id).await;
    match entity_response {
        Ok(response) => Ok(message.reply(state_to_string(response.state.unwrap()), MessageType::TEXT)?),
        Err(error) => {
            let err_msg = format!("An error occurred while fetching the entity ({}): {}", entity_id, error);
            error!("{}", err_msg);
            Err(err_msg.into())
        }
    }
}

async fn set_state_handler(client: &Client, message: &Message) -> Result<(String, Message), Box<dyn Error>> {
    let split = message.text.split(" ").collect::<Vec<&str>>();
    if split.len() != 2 {
        let err_msg = format!("Wrong format: {}", message.text);
        error!("{}", err_msg);
        return Err(err_msg.into());
    }
    let entity_id = split[0].to_string();
    let state = split[1].to_string();
    let get_entity_response = client.get_states_of_entity(&*entity_id).await;
    if let Err(error) = get_entity_response {
        let err_msg = format!("An error occurred while fetching the entity ({}): {}", entity_id, error);
        error!("{}", err_msg);
        return Err(err_msg.into());
    }
    let entity_response = get_entity_response?;
    let params = StateParams {
        entity_id: entity_id.clone(),
        state,
        attributes: entity_response.attributes,
    };
    match client.post_states(params).await {
        Ok(response) => Ok(message.reply(state_to_string(response.state.unwrap()), MessageType::TEXT)?),
        Err(error) => {
            let err_msg = format!("An error occurred while fetching the entity ({}): {}", entity_id, error);
            error!("{}", err_msg);
            Err(err_msg.into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut module = InterfaceModule::new(MODULE_NAME).await.expect("An error occurred while fetching the module");
    module.listen(MODULE_NAME).await.expect("An error occurred while listening");
    let base_url = module.config.get_module_value("url").expect("Missing home assistant url");

    let token = module.config.get_module_value("token").expect("Missing home assistant token");

    let client = Client::new(&*base_url, &*token).expect("An error occurred while creating the client");
    loop {
        let (topic, message) = module.receive().await.expect("An error occurred while fetching the module");
        let res = match topic.as_str() {
            SET_STATE_TOPIC => set_state_handler(&client, &message).await,
            GET_STATE_TOPIC => get_state_handler(&client, &message).await,
            _ => {
                warn!("Unknown topic: {}", topic);
                Err("Unknown topic".into())
            }
        };
        if let Ok((response_topic, reply)) = res {
            module.send(&response_topic, &reply).await.expect("An error occurred while sending a reply");
        }
    }
}
