use rand::{thread_rng, Rng};
use shared::{process::Process, Bot, GameError, GameResult, PresignedRequest, WhichBot};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt},
    process::Command,
    try_join,
};

use crate::poker::game::GameState;

pub mod sandbox;
pub async fn download_and_run<T: Into<String>, U: Into<String>, V: Into<PathBuf>>(
    bot: U,
    bot_path: V,
    bot_bucket: T,
    s3_client: &aws_sdk_s3::Client,
) -> Result<Process, GameError> {
    let bot_path: PathBuf = bot_path.into();
    shared::s3::download_file(
        &bot.into(),
        &bot_path.join("bot.zip"),
        &bot_bucket.into(),
        &s3_client,
    )
    .await?;

    log::debug!("Bot downloaded");
    Command::new("unzip")
        .arg(&bot_path.join("bot.zip"))
        .current_dir(&bot_path)
        .spawn()?
        .wait()
        .await?;
    log::debug!("Bot unzipped to {:?}", bot_path);

    let bot_json: Bot = async {
        let json = fs::read_to_string(&bot_path.join("bot/bot.json")).await?;
        if let Ok(bot) = serde_json::from_str::<Bot>(&json) {
            return Ok(bot);
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to parse bot.json",
        ))
    }
    .await?;
    log::debug!("Read json");

    shared::process::Process::sh_configured(bot_json.run, move |command| {
        command.current_dir(&bot_path.join("bot"))
    })
    .await
    .map_err(|e| {
        log::error!("Error running bot: {}", e);
        GameError::InternalError
    })
}

pub async fn run_game(
    bot_a: &String,
    bot_b: &String,
    s3_client: &aws_sdk_s3::Client,
    task_id: &String,
    rounds: usize,
    game_path: &mut PathBuf,
) -> GameResult {
    // create tmp directory
    // doesn't have the same id as the task
    let game_id = format!("{:x}", rand::thread_rng().gen::<u32>());
    let tmp_dir = Path::new("/tmp").join(&game_id);
    *game_path = tmp_dir.clone();
    log::debug!("Playing {} against {}", bot_a, bot_b);
    log::debug!("Running game {} with local id {}.", task_id, game_id);
    let bot_bucket = std::env::var("COMPILED_BOT_S3_BUCKET").map_err(|e| {
        log::error!("Error getting COMPILED_BOT_S3_BUCKET: {}", e);
        GameError::InternalError
    })?;
    log::debug!("Bot bucket: {}", bot_bucket);

    // download bots from s3
    log::debug!("Making bot directories");
    let bot_a_path = tmp_dir.join("bot_a");
    fs::create_dir_all(&bot_a_path).await.map_err(|e| {
        log::error!("Error creating bot_a directory: {}", e);
        shared::GameError::InternalError
    })?;
    let bot_b_path = tmp_dir.join("bot_b");
    fs::create_dir_all(&bot_b_path).await.map_err(|e| {
        log::error!("Error creating bot_b directory: {}", e);
        shared::GameError::InternalError
    })?;
    log::debug!("Downloading bots from aws");
    let (bot_a, bot_b) = try_join!(
        download_and_run(bot_a, bot_a_path, &bot_bucket, s3_client),
        download_and_run(bot_b, bot_b_path, &bot_bucket, s3_client)
    )?;

    // run game
    let mut game = Game::new(
        bot_a,
        bot_b,
        game_id,
        Duration::from_secs(1),
        tokio::fs::File::create(tmp_dir.join("logs")).await?,
    );

    game.play(rounds).await
}

pub struct Game {
    bot_a: Process,
    bot_b: Process,
    stacks: [u32; 2],
    initial_stacks: [u32; 2],
    button: usize,
    id: String,
    timeout: Duration,
    logs: tokio::fs::File,
}
impl Game {
    pub fn new(
        bot_a: Process,
        bot_b: Process,
        id: String,
        timeout: Duration,
        logs: tokio::fs::File,
    ) -> Self {
        Self {
            bot_a,
            bot_b,
            stacks: [50, 50],
            initial_stacks: [50, 50],
            button: 0,
            timeout,
            id,
            logs,
        }
    }

    async fn print_position(&mut self, which_bot: WhichBot) -> Result<(), GameError> {
        let bot = match which_bot {
            WhichBot::BotA => &mut self.bot_a,
            WhichBot::BotB => &mut self.bot_b,
        };

        self.logs
            .write(format!("{} <<< P", which_bot).as_bytes())
            .await?;
        bot.input.write(b"P").await?;
        let position = format!(
            " {}",
            match which_bot {
                WhichBot::BotA => self.button,
                WhichBot::BotB => (self.button + 1) % 2,
            }
        );
        self.logs.write(position.as_bytes()).await?;
        bot.input.write(position.as_bytes()).await?;
        self.logs.write(b"\n").await?;
        bot.input.write(b"\n").await?;
        bot.input.flush().await?;
        Ok(())
    }

    async fn print_round_end(&mut self, which_bot: WhichBot) -> Result<(), shared::GameError> {
        let bot = match which_bot {
            WhichBot::BotA => &mut self.bot_a,
            WhichBot::BotB => &mut self.bot_b,
        };

        bot.input.write(b"E\n").await?;
        bot.input.flush().await?;
        self.logs
            .write(format!("{} <<< E\n", which_bot).as_bytes())
            .await?;
        Ok(())
    }

    async fn print_cards(
        &mut self,
        which_bot: WhichBot,
        state: &GameState,
    ) -> Result<(), shared::GameError> {
        let bot = match which_bot {
            WhichBot::BotA => &mut self.bot_a,
            WhichBot::BotB => &mut self.bot_b,
        };

        self.logs
            .write(format!("{} <<< C", which_bot).as_bytes())
            .await?;
        bot.input.write(b"C").await?;

        for card in state.player_states[match which_bot {
            WhichBot::BotA => 0,
            WhichBot::BotB => 1,
        }]
        .hole_cards
        .iter()
        {
            let card = format!(" {}", card);
            bot.input.write(card.as_bytes()).await?;
            self.logs.write(card.as_bytes()).await?;
        }
        for card in state.community_cards.iter() {
            let card = format!(" {}", card);
            bot.input.write(card.as_bytes()).await?;
            self.logs.write(card.as_bytes()).await?;
        }
        self.logs.write(b"\n").await?;
        bot.input.write(b"\n").await?;
        bot.input.flush().await?;

        Ok(())
    }

    async fn play_round(&mut self) -> Result<(), shared::GameError> {
        let mut rng = thread_rng();
        let mut stacks = self.stacks;
        if self.button == 1 {
            stacks = [stacks[1], stacks[0]];
        }
        let mut state =
            crate::poker::game::GameState::new(&stacks, GameState::get_shuffled_deck(&mut rng));

        log::debug!("Game state: {:?}. ", state);

        let mut round = None;

        self.print_position(WhichBot::BotA).await.map_err(|_| {
            log::info!("Failed to print position to bot A.");
            GameError::RunTimeError(WhichBot::BotA)
        })?;

        self.print_position(WhichBot::BotB).await.map_err(|_| {
            log::info!("Failed to print position to bot B.");
            GameError::RunTimeError(WhichBot::BotB)
        })?;
        loop {
            self.stacks = if self.button == 1 {
                [state.get_stack(true), state.get_stack(false)]
            } else {
                [state.get_stack(false), state.get_stack(true)]
            };

            if state.round_over() {
                log::debug!("Round ended.");
                self.print_round_end(WhichBot::BotA).await.map_err(|_| {
                    log::info!("Failed to print round end to bot A.");
                    GameError::RunTimeError(WhichBot::BotA)
                })?;

                self.print_round_end(WhichBot::BotB).await.map_err(|_| {
                    log::info!("Failed to print round end to bot B.");
                    GameError::RunTimeError(WhichBot::BotB)
                })?;
                break;
            }
            // Print community cards to both bots
            if round != Some(state.round) {
                log::debug!("Printing community cards.");
                round = Some(state.round);
                self.print_cards(WhichBot::BotA, &state)
                    .await
                    .map_err(|_| {
                        log::info!("Failed to print community cards to bot A.");
                        GameError::RunTimeError(WhichBot::BotA)
                    })?;
                self.print_cards(WhichBot::BotB, &state)
                    .await
                    .map_err(|_| {
                        log::info!("Failed to print community cards to bot B.");
                        GameError::RunTimeError(WhichBot::BotB)
                    })?;
            }
            // Assume state.whose_turn() is not None
            let whose_turn: WhichBot =
                if state.whose_turn().ok_or(GameError::InternalError)? == self.button {
                    WhichBot::BotA
                } else {
                    WhichBot::BotB
                };

            let target_bot = match whose_turn {
                WhichBot::BotA => &mut self.bot_a,
                WhichBot::BotB => &mut self.bot_b,
            };

            // write current game state to the bots stream
            log::debug!("Writing current state.");
            let status = format!(
                "S {} {} {} {} {}\n",
                state.target_push,
                state.player_states[0].pushed,
                state.player_states[1].pushed,
                state.player_states[0].stack,
                state.player_states[1].stack,
            );
            self.logs
                .write(format!("{} <<< {}", whose_turn, status).as_bytes())
                .await?;
            target_bot
                .input
                .write(status.as_bytes())
                .await
                .map_err(|_| {
                    log::info!("Failed to write current state to bot {:?}.", whose_turn);
                    GameError::RunTimeError(whose_turn)
                })?;
            if let Err(e) = target_bot.input.flush().await {
                log::info!("Failed to flush input stream of bot {:?}.", whose_turn);
                return Err(GameError::RunTimeError(whose_turn));
            }

            {
                let status = target_bot.status.clone();
                let status = status.lock().await;
                if status.is_some() {
                    return Err(shared::GameError::RunTimeError(whose_turn.clone()));
                }
            }
            log::debug!("Reading action from {:?}.", whose_turn);
            let mut line: String = Default::default();

            let len = tokio::time::timeout(self.timeout, target_bot.output.read_line(&mut line))
                .await
                .map_err(|e| shared::GameError::TimeoutError(whose_turn.clone()))?
                .map_err(|e| shared::GameError::RunTimeError(whose_turn.clone()))?;
            self.logs
                .write(format!("{} >>> {}\n", whose_turn, line).as_bytes())
                .await?;
            log::debug!("Reading action from {:?}.", line);
            state = state
                .post_action(
                    parse_action(line.trim())
                        .map_err(|e| shared::GameError::InvalidActionError(whose_turn.clone()))?,
                )
                .map_err(|e| shared::GameError::InvalidActionError(whose_turn.clone()))?;
        }

        Ok(())
    }
    /// Play a game of poker, returning a [shared::GameResult]
    pub async fn play(&mut self, rounds: usize) -> shared::GameResult {
        log::debug!("Playing game {} with {} rounds", self.id, rounds);

        log::info!("Clients connected for {}", self.id);
        for i in 0..rounds {
            self.logs
                .write(format!("Engine >>> round {}/{}", i + 1, rounds).as_bytes())
                .await?;
            log::debug!("Playing round. Current stacks: {:?}.", self.stacks);
            if self.stacks[0] == 0 || self.stacks[1] == 0 {
                break;
            }
            self.play_round().await?;
            self.button = 1 - self.button;
        }
        Ok(shared::GameStatus::ScoreChanged(
            i32::try_from(self.stacks[0]).unwrap() - i32::try_from(self.initial_stacks[0]).unwrap(),
        ))
    }
}

fn parse_action<T: AsRef<str>>(
    line: T,
) -> Result<crate::poker::game::Action, shared::GameActionError> {
    let line = line.as_ref();
    Ok(match line.as_ref() {
        "X" => crate::poker::game::Action::Check,
        "F" => crate::poker::game::Action::Fold,
        "C" => crate::poker::game::Action::Call,
        _ => {
            if line.chars().nth(0) != Some('R') {
                Err(shared::GameActionError::CouldNotParse)?;
            }
            let amount = line[1..]
                .parse::<u32>()
                .map_err(|_| shared::GameActionError::CouldNotParse)?;
            crate::poker::game::Action::Raise { amt: amount }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::parse_action;
    #[test]
    fn parse_action_check() {
        assert_eq!(
            parse_action(&"X".to_owned()).unwrap(),
            crate::poker::game::Action::Check
        );
    }

    #[test]
    fn parse_action_fold() {
        assert_eq!(
            parse_action(&"F".to_owned()).unwrap(),
            crate::poker::game::Action::Fold
        );
    }

    #[test]
    fn parse_action_call() {
        assert_eq!(
            parse_action(&"C".to_owned()).unwrap(),
            crate::poker::game::Action::Call
        );
    }

    #[test]
    fn parse_action_raise() {
        assert_eq!(
            parse_action(&"R1234".to_owned()).unwrap(),
            crate::poker::game::Action::Raise { amt: 1234 }
        );
    }

    #[test]
    fn parse_action_raise_invalid() {
        assert!(parse_action(&"R".to_owned()).is_err());
    }

    #[test]
    fn parse_action_raise_invalid2() {
        assert!(parse_action(&"R1234a".to_owned()).is_err());
    }

    #[test]
    fn parse_action_raise_invalid3() {
        assert!(parse_action(&"R-1234".to_owned()).is_err());
    }

    #[test]
    fn parse_action_raise_invalid4() {
        assert!(parse_action(&"R-1".to_owned()).is_err());
    }

    #[test]
    fn parse_action_raise_invalid5() {
        assert!(parse_action(&"R1234.0".to_owned()).is_err());
    }

    #[test]
    fn parse_action_raise_invalid6() {
        assert!(parse_action(&"B".to_owned()).is_err());
    }
}
