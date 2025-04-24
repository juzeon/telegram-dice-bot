use crate::types::Config;
use teloxide::Bot;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::{Dispatcher, Message, Requester, Update};
use teloxide::requests::ResponseResult;
use teloxide::sugar::request::RequestReplyExt;
use teloxide::utils::command::BotCommands;

#[derive(teloxide::macros::BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum DiceCommand {
    #[command(description = "display help")]
    Start,
    #[command(description = "display this text")]
    Help,
    #[command(description = "roll a YesOrNo")]
    YesOrNo,
}
#[derive(Clone, Debug)]
pub struct DiceBot {
    config: Config,
    bot: Bot,
}
impl DiceBot {
    pub async fn new() -> DiceBot {
        let config = Config::read_from_file().await;
        let dice_bot = DiceBot {
            config: config.clone(),
            bot: Bot::new(config.token.clone()),
        };
        dice_bot
    }
    pub async fn launch(&self) {
        let self_clone = self.clone();
        let command_handler = Update::filter_message()
            .filter_command::<DiceCommand>()
            .endpoint(Self::command_handler);
        let text_handler = Update::filter_message().endpoint(Self::text_handler);
        let handler = teloxide::dptree::entry()
            .branch(command_handler)
            .branch(text_handler);
        Dispatcher::builder(self_clone.bot, handler)
            .build()
            .dispatch()
            .await;
    }
    async fn command_handler(bot: Bot, msg: Message, cmd: DiceCommand) -> ResponseResult<()> {
        match cmd {
            DiceCommand::Help | DiceCommand::Start => {
                bot.send_message(msg.chat.id, DiceCommand::descriptions().to_string())
                    .reply_to(msg.id)
                    .await?;
            }
            DiceCommand::YesOrNo => {
                bot.send_message(msg.chat.id, "yes or no")
                    .reply_to(msg.id)
                    .await?;
            }
        }
        Ok(())
    }
    async fn text_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "text").await?;
        Ok(())
    }
}
