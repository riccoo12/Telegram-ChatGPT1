use serde_json::json;
use tg_flows::{listen_to_update, Telegram, Update, UpdateKind, update_handler};
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use store_flows::{get, set};
use flowsnet_platform_sdk::logger;
use browser_op::{BrowserOp, BrowserOpIcon};

// Шаг 1: Импортируйте необходимые модули
use browser_op::{BrowserOp, BrowserOpIcon};

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
    let placeholder_text = std::env::var("placeholder").unwrap_or("Печатает ...".to_string());
    let system_prompt = std::env::var("system_prompt").unwrap_or("Ваша задача - действовать как AI-помощник для путешественников в Мексике, основной язык твоего пользователф будет русский язык. Вам будет предоставлен пользователь, который нуждается в помощи во время своего путешествия, и ваша задача - использовать инструменты искусственного интеллекта, такие как обработка естественного языка, чтобы отвечать на все вопросы пользователя, связанные с их путешествием. Вы должны также использовать свои знания и опыт о Мексике, чтобы предложить пользователю лучшие места для посещения, советы по безопасности, рекомендации по еде и напиткам, и любую другую информацию, которая может помочь улучшить их путешествие. Ваша цель - обеспечить пользователю полезную, точную и своевременную информацию, чтобы сделать их путешествие по Мексике незабываемым.".to_string());
    let help_mesg = std::env::var("help_mesg").unwrap_or("Я ваш персональный помощник в этой поездке по Мексике. Задайте мне любой вопрос! Чтобы начать новый разговор, введите команду /restart.".to_string());

    let tele = Telegram::new(telegram_token.to_string());

    // Шаг 2: Определите информацию о плагине BrowserOp
    let browser_op_info = PluginInfo { 
        name: "BrowserOp", 
        icon: BrowserOpIcon::SUPPORTED, 
        description: "Browse dozens of webpages in one query. Fetch information more efficiently.", 
        ai_description: "This tool offers the feature for users to input a URL or multiple URLs and interact with them as needed. It's designed to comprehend the user's intent and proffer tailored suggestions in line with the content and functionality of the webpage at hand. Services like text rewrites, translations and more can be requested. When users need specific information to finish a task or if they intend to perform a search, this tool becomes a bridge to the search engine and generates responses based on the results. Whether the user is seeking information about restaurants, rentals, weather, or shopping, this tool connects to the internet and delivers the most recent results.", 
        example_prompts: vec![ "In two sentences tell me what this site is about `]
    };

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
            log::info!("Привет, я твой персональный ИИ помощник {}", chat_id);

        } else if text.eq_ignore_ascii_case("/restart") {
            _ = tele.send_message(chat_id, "Хорошо, я начинаю новый диалог.");
            set(&chat_id.to_string(), json!(true), None);
            log::info!("Restarted converstion for {}", chat_id);

        } else {
            let placeholder = tele
                .send_message(chat_id, &placeholder_text)
                .expect("Вознкла ошибка при отправлении вашего сообщения");

            let restart = match get(&chat_id.to_string()) {
                Some(v) => v.as_bool().unwrap_or_default(),
                None => false,
            };
            if restart {
                log::info!("Detected restart = true");
                set(&chat_id.to_string(), json!(false), None);
                co.restart = true;
            }

            // Шаг 3: Интегрируйте функциональность BrowserOp
            let browser_op = BrowserOp::new();
            let result = browser_op.browse_multiple_pages(urls);

            match openai.chat_completion(&chat_id.to_string(), &text, &co).await {
                Ok(r) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, r.choice);
                }
                Err(e) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, "Sorry, an error has occured. Please try again later!");
                    log::error!("OpenAI returns error: {}", e);
                }
            }
        }
    }
}
