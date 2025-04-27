use crate::types::Config;
use anyhow::{Context, bail};
use rand::prelude::StdRng;
use rand::{Rng, RngCore, SeedableRng};
use regex::{Captures, Regex};
use std::error::Error;
use std::num::ParseIntError;
use std::ops::{Deref, Range, RangeInclusive};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use teloxide::Bot;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Dispatcher, Message, Requester, Update};
use teloxide::requests::{Output, ResponseResult};
use teloxide::sugar::request::RequestReplyExt;
use teloxide::types::{ParseMode, ReplyParameters};
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

static DICE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d*)[dD](\d+)((?:[+-]\d+)*)(?: +(.*))?$").unwrap());
static DICE_FIXES_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([+-]\d+)").unwrap());
#[derive(teloxide::macros::BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "骰子语法：
- <code>d10 投1个10面骰，空格之后可以写注释（也可以不写）</code>
- <code>2d100 投两个100面骰</code>
- <code>d10+3 投1个10面骰，修正值+3</code>
- <code>2d10+3 投2个10面骰，每个修正值+3</code>
- <code>2d10-3 也可以-3</code>
- <code>2d10+3+5 对每个骰子赋予不同修正值</code>
- <code>3d10+30-50+40</code>

命令：
"
)]
enum DiceCommand {
    #[command(description = "显示帮助")]
    Start,
    #[command(description = "显示帮助")]
    Help,
    #[command(description = "选一个YesOrNo")]
    YesOrNo(String),
}
#[derive(Clone)]
pub struct DiceBot {
    rng: Arc<Mutex<StdRng>>,
    config: Config,
    bot: Bot,
}
enum Fix {
    Add(usize),
    Sub(usize),
}
impl Fix {
    fn get_value(&self) -> usize {
        match self {
            Fix::Add(num) => *num,
            Fix::Sub(num) => *num,
        }
    }
}
impl FromStr for Fix {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let sign = chars.next().context("empty string")?;
        let num_str = chars.as_str(); // Get remaining string after sign

        if num_str.is_empty() {
            bail!("missing number after sign");
        }

        let num = num_str.parse::<usize>()?;

        match sign {
            '+' => Ok(Self::Add(num)),
            '-' => Ok(Self::Sub(num)),
            _ => bail!("invalid sign character '{}'", sign),
        }
    }
}
impl DiceBot {
    pub async fn new() -> DiceBot {
        let config = Config::read_from_file().await;
        let mut rng = StdRng::from_os_rng();
        if config.real_random {
            match Self::get_real_random_rng().await {
                Ok(r) => {
                    info!("Use real random rng");
                    rng = r;
                }
                Err(err) => {
                    warn!("Cannot get real random rng: {:#}", err);
                }
            }
        }
        let dice_bot = DiceBot {
            rng: Arc::new(Mutex::new(rng)),
            config: config.clone(),
            bot: Bot::new(config.token.clone()),
        };
        info!("Created bot");
        dice_bot
    }
    async fn get_real_random_rng() -> anyhow::Result<StdRng> {
        let text = reqwest::Client::builder()
            .timeout(Duration::from_secs(10)).build().unwrap()
            .get("https://www.random.org/integers/?num=1&min=1&max=1000000000&col=1&base=10&format=plain&rnd=new")
            .send().await?.text().await?.trim().to_string();
        debug!(text, "From random.org");
        let u = text.parse::<u64>()?;
        Ok(StdRng::seed_from_u64(u))
    }
    pub async fn launch(&self) {
        let handle_handler_result =
            async |bot: &Bot, msg: &Message, result: anyhow::Result<()>| match result {
                Ok(_) => result,
                Err(err) => {
                    let _ = Self::reply(bot, msg, &format!("{:#}", err)).await;
                    Err(err)
                }
            };
        let self_clone = self.clone();
        let command_handler = Update::filter_message()
            .filter_command::<DiceCommand>()
            .endpoint(move |bot: Bot, msg: Message, cmd: DiceCommand| {
                let inner_self_clone = self_clone.clone();
                async move {
                    let result = inner_self_clone.command_handler(&bot, &msg, cmd).await;
                    handle_handler_result(&bot, &msg, result).await
                }
            });
        let self_clone = self.clone();
        let text_handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
            let inner_self_clone = self_clone.clone();
            async move {
                let result = inner_self_clone.text_handler(&bot, &msg).await;
                handle_handler_result(&bot, &msg, result).await
            }
        });
        let handler = teloxide::dptree::entry()
            .branch(command_handler)
            .branch(text_handler);
        let self_clone = self.clone();
        Dispatcher::builder(self_clone.bot, handler)
            .build()
            .dispatch()
            .await;
    }
    async fn command_handler(
        &self,
        bot: &Bot,
        msg: &Message,
        cmd: DiceCommand,
    ) -> anyhow::Result<()> {
        match cmd {
            DiceCommand::Help | DiceCommand::Start => {
                bot.send_message(msg.chat.id, DiceCommand::descriptions().to_string())
                    .parse_mode(ParseMode::Html)
                    .reply_to(msg.id)
                    .await?;
            }
            DiceCommand::YesOrNo(title) => {
                let prefix = if !title.is_empty() {
                    format!("<b>{title}：</b>\n\n")
                } else {
                    "".into()
                };
                let res = if self.get_random(0..=1).await == 0 {
                    "Yes!"
                } else {
                    "No!"
                };
                Self::reply(bot, msg, &format!("{prefix}{res}")).await?;
            }
        }
        Ok(())
    }
    async fn reply(bot: &Bot, msg: &Message, text: &str) -> anyhow::Result<()> {
        bot.send_message(msg.chat.id, text)
            .parse_mode(ParseMode::Html)
            .reply_to(msg.id)
            .await?;
        Ok(())
    }
    async fn text_handler(&self, bot: &Bot, msg: &Message) -> anyhow::Result<()> {
        debug!(?msg, "Message");
        let mut text = match msg.text() {
            None => return Ok(()),
            Some(t) => t,
        };
        if self.config.prefix.as_str() != "" {
            match text.strip_prefix(self.config.prefix.as_str()) {
                None => {
                    if msg.chat.is_private() {
                        Self::reply(bot, msg, "骰子语法不正确，请使用 /help 查看帮助").await?;
                    }
                    return Ok(());
                }
                Some(stripped) => {
                    text = stripped;
                }
            }
        }
        let caps = match DICE_RE.captures(text) {
            None => {
                if msg.chat.is_private() || self.config.prefix.as_str() != "" {
                    Self::reply(bot, msg, "骰子语法不正确，请使用 /help 查看帮助").await?;
                }
                return Ok(());
            }
            Some(c) => c,
        };
        let count = caps
            .get(1)
            .map(|x| x.as_str())
            .unwrap_or("1")
            .parse::<usize>()
            .unwrap_or(1);
        let dimension = caps
            .get(2)
            .map(|x| x.as_str())
            .unwrap_or("10")
            .parse::<usize>()
            .unwrap_or(10);
        let fixes = caps.get(3).map(|x| x.as_str()).unwrap_or("");
        let comment = caps.get(4).map(|x| x.as_str()).unwrap_or("");
        let mut fix_caps = None;
        if count > 100 {
            bail!("骰子个数不能大于100");
        }
        if !fixes.is_empty() {
            fix_caps = Some(DICE_FIXES_RE.captures_iter(fixes).collect::<Vec<_>>());
            let len = fix_caps.as_ref().unwrap().len();
            if len != count && len != 1 {
                bail!("修正值和骰子面数不匹配");
            }
        }
        info!(?caps, count, dimension, fixes, comment, ?fix_caps, "Dice");
        let mut res_arr: Vec<String> = vec![];
        for i in 1..=count {
            let roll = self.get_random(1..=dimension).await;
            let prefix = if count != 1 {
                &format!("第{}个骰子：", i)
            } else {
                ""
            };
            if let Some(fix_caps) = fix_caps.as_ref() {
                let fix: Fix = fix_caps[if fix_caps.len() == 1 { 0 } else { i - 1 }][1].parse()?;
                let final_value: isize;
                let sign;
                match fix {
                    Fix::Add(num) => {
                        final_value = (roll + num) as isize;
                        sign = "+";
                    }
                    Fix::Sub(num) => {
                        final_value = (roll as isize) - (num as isize);
                        sign = "-";
                    }
                };
                res_arr.push(format!(
                    "{}{roll} {sign} {} = {final_value}",
                    prefix,
                    fix.get_value()
                ))
            } else {
                res_arr.push(format!("{}{}", prefix, roll))
            }
        }
        let prefix = if comment.is_empty() {
            ""
        } else {
            &format!("<b>{comment}：</b>\n\n")
        };
        Self::reply(bot, msg, &format!("{prefix}{}", res_arr.join("\n"))).await?;
        Ok(())
    }
    async fn get_random(&self, range: RangeInclusive<usize>) -> usize {
        self.rng.lock().await.random_range(range)
    }
}
