use serde_json::json;
use tg_flows::{listen_to_update, Telegram, Update, UpdateKind, update_handler};
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use store_flows::{get, set};
use flowsnet_platform_sdk::logger;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let telegram_token = std::env::var("telegram_token").unwrap();
    listen_to_update(telegram_token).await;
}

#[update_handler]
async fn handler(update: Update) {
    logger::init();
    let telegram_token = std::env::var("telegram_token").unwrap();
    let placeholder_text = std::env::var("placeholder").unwrap_or("Печатает ваш ...".to_string());
    let system_prompt = std::env::var("system_prompt").unwrap_or("Ваша задача - действовать как AI-помощник для путешественников из снг, говорящих на русском языке в Мексике. Вам будет предоставлен пользователь, который нуждается в помощи во время своего путешествия, и ваша задача - использовать инструменты искусственного интеллекта, такие как обработка естественного языка, чтобы отвечать на все вопросы пользователя, связанные с их путешествием, знать курс обмена валют по направлениям Рубли, Американские доллары и Мексиканские песо. Вы должны также использовать свои знания и опыт о Мексике, чтобы предложить пользователю лучшие места для посещения, советы по безопасности(мы находимся в опасной стране, а именно Мексика), рекомендации по еде и напиткам, и любую другую информацию, которая может помочь улучшить их путешествие. Ваша цель - обеспечить пользователю полезную, точную и своевременную информацию, чтобы сделать их путешествие по Мексике незабываемым.".to_string());
    let help_mesg = std::env::var("help_mesg").unwrap_or("Я ваш ИИ - помощник пока вы находитесь в Мексике. Задайте мне любой вопрос! Чтобы начать новый разговор, введите команду /restart.".to_string());

    let tele = Telegram::new(telegram_token.to_string());

    if let UpdateKind::Message(msg) = update.kind {
        let chat_id = msg.chat.id;
        log::info!("Received message from {}", chat_id);

        let mut openai = OpenAIFlows::new();
        openai.set_retry_times(3);
        let mut co = ChatOptions::default();
        // co.model = ChatModel::GPT4;
        co.model = ChatModel::GPT35Turbo;
        co.restart = false;
        co.system_prompt = Some(&system_prompt);

        let text = msg.text().unwrap_or("");
        if text.eq_ignore_ascii_case("/help") {
            _ = tele.send_message(chat_id, &help_mesg);

        } else if text.eq_ignore_ascii_case("/start") {
            _ = tele.send_message(chat_id, &help_mesg);
            set(&chat_id.to_string(), json!(true), None);
            log::info!("Started converstion for {}", chat_id);

        } else if text.eq_ignore_ascii_case("/restart") {
            _ = tele.send_message(chat_id, "Ok, I am starting a new conversation.");
            set(&chat_id.to_string(), json!(true), None);
            log::info!("Restarted converstion for {}", chat_id);

        } else {
            let placeholder = tele
                .send_message(chat_id, &placeholder_text)
                .expect("Возникает ошибка при отправке сообщения в Telegram");

            let restart = match get(&chat_id.to_string()) {
                Some(v) => v.as_bool().unwrap_or_default(),
                None => false,
            };
            if restart {
                log::info!("Detected restart = true");
                set(&chat_id.to_string(), json!(false), None);
                co.restart = true;
            }

            match openai.chat_completion(&chat_id.to_string(), &text, &co).await {
                Ok(r) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, r.choice);
                }
                Err(e) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, "Извините, произошла ошибка. Пожалуйста, повторите попытку позже!");
                    log::error!("OpenAI returns error: {}", e);
                }
            }
        }
    }
}
