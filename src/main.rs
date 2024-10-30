use std::future::Future;
use std::path::Path;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use rustemon::Follow;
use rustemon::client::{CACacheManager, RustemonClient, RustemonClientBuilder};
use rustemon::model::resource::{Name, NamedApiResource};
use rustemon::model::utility::Language;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Parser)]
#[command(about, author, version, long_about = None)]
struct Config {
    /// The directory within which to cache data.
    #[arg(long = "cache-dir", default_value = ".cache")]
    cache_dir: Option<Box<Path>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Parser)]
struct Arguments {
    #[command(flatten)]
    config: Config,
    #[command(subcommand)]
    command: Command,
}

#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Subcommand)]
#[command(about, author, long_about = None)]
enum Command {
    /// Saves the command's arguments to a file.
    SaveConfig {
        /// The output file.
        #[arg(default_value = concat!(env!("CARGO_PKG_NAME"), ".toml"))]
        path: Box<Path>,
    },
    /// Searches for the given data.
    Search {
        /// The search type.
        kind: SearchType,
        /// The search string.
        name: Box<str>,
    },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, ValueEnum)]
enum SearchType {
    Pokemon,
    Ability,
    Move,
    Item,
}

fn save_config(arguments: Arguments) -> Result<()> {
    let Command::SaveConfig { path } = arguments.command else { unreachable!() };
    let content = toml::to_string_pretty(&arguments.config)?;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    std::fs::write(path, content).map_err(Into::into)
}

macro_rules! write {
    ($buffer:expr, $($args:tt)+) => {
        $buffer.write_all(format!($($args)+).as_bytes())
    };
}

macro_rules! writeln {
    ($buffer:expr, $($args:tt)+) => {
        async {
            $buffer.write_all(format!($($args)+).as_bytes()).await?;
            $buffer.write_u8(b'\n').await
        }
    };
}

async fn search(arguments: Arguments, client: RustemonClient) -> Result<()> {
    #[inline]
    fn linear_find<T, U>(list: &[T], find: impl Fn(&&T) -> bool, map: impl Copy + FnOnce(&T) -> &U) -> &U {
        list.iter().find(find).map_or_else(|| map(&list[0]), |v| map(v))
    }

    #[inline]
    fn english(list: &[Name]) -> &str {
        linear_find(list, |v| v.language.name == "en", |v| &v.name)
    }

    #[inline]
    fn english_by<T>(list: &[T], map: impl Fn(&T) -> &NamedApiResource<Language>) -> &T {
        linear_find(list, |v| map(v).name == "en", |v| v)
    }

    let Command::Search { kind, name } = arguments.command else { unreachable!() };
    let api_name = name.replace(' ', "-").to_lowercase();
    let mut stdout = tokio::io::stdout();

    match kind {
        SearchType::Pokemon => todo!(),
        SearchType::Move => {
            let r#move = match rustemon::moves::move_::get_by_name(&api_name, &client).await {
                Ok(r#move) => r#move,
                Err(error) => bail!("failed to resolve move '{name}' - {error}"),
            };

            let move_name = english(&r#move.names);
            let move_gen = r#move.generation.follow(&client).await?;
            let move_gen = english(&move_gen.names);
            let move_effect = &english_by(&r#move.effect_entries, |v| &v.language).effect;

            writeln!(stdout, "{move_name} ({move_gen})").await?;

            let move_class = r#move.damage_class.follow(&client).await?;
            let move_class = english(&move_class.names);
            let move_class = move_class
                .chars()
                .take(1)
                .map(|c| c.to_ascii_uppercase())
                .chain(move_class.chars().skip(1))
                .collect::<String>();

            writeln!(stdout, "\nClass: {move_class}").await?;

            let move_type = r#move.type_.follow(&client).await?;
            let move_type = english(&move_type.names);

            writeln!(stdout, "Type: {move_type}").await?;

            if let Some(move_pp) = r#move.pp {
                writeln!(stdout, "PP: {move_pp}").await?;
            } else {
                writeln!(stdout, "PP: --").await?;
            }

            if let Some(move_power) = r#move.power {
                writeln!(stdout, "Power: {move_power}").await?;
            } else {
                writeln!(stdout, "Power: --").await?;
            }

            if let Some(move_accuracy) = r#move.accuracy {
                writeln!(stdout, "Accuracy: {move_accuracy}%").await?;
            } else {
                writeln!(stdout, "Accuracy: --").await?;
            }

            if r#move.priority != 0 {
                writeln!(stdout, "Priority: {}", r#move.priority).await?;
            }

            let move_target = r#move.target.follow(&client).await?;
            let move_target = english(&move_target.names);

            writeln!(stdout, "Target: {move_target}").await?;
            writeln!(stdout, "\n---\n\n{move_effect}").await?;
        }
        SearchType::Ability => {
            let ability = match rustemon::pokemon::ability::get_by_name(&api_name, &client).await {
                Ok(ability) => ability,
                Err(error) => bail!("failed to resolve ability '{name}' - {error}"),
            };

            let ability_name = english(&ability.names);
            let ability_effect = &english_by(&ability.effect_entries, |v| &v.language).effect;
            let ability_gen = ability.generation.follow(&client).await?;
            let ability_gen = english(&ability_gen.names);

            writeln!(stdout, "{ability_name} ({ability_gen})\n\n---\n\n{ability_effect}").await?;
        }
        SearchType::Item => {
            let item = match rustemon::items::item::get_by_name(&api_name, &client).await {
                Ok(item) => item,
                Err(error) => bail!("failed to resolve item '{name}' - {error}"),
            };

            let item_name = english(&item.names);
            let item_category = item.category.follow(&client).await?;
            let item_category = english(&item_category.names);

            writeln!(stdout, "{item_name} ({item_category})\n\n---\n").await?;

            if let Some(effect) = item.fling_effect {
                let effect = effect.follow(&client).await?;

                write!(stdout, "Thrown with fling").await?;

                if let Some(power) = item.fling_power {
                    write!(stdout, " ({power} power)").await?;
                }

                let effect = &english_by(&effect.effect_entries, |v| &v.language).effect;

                writeln!(stdout, "\n:   {effect}\n").await?;
            }

            let item_effect = &english_by(&item.effect_entries, |v| &v.language).effect;

            writeln!(stdout, "{item_effect}").await?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    #[inline]
    fn run<T, F>(cache_manager: CACacheManager, future: impl FnOnce(RustemonClient) -> F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let client = RustemonClientBuilder::default().with_manager(cache_manager).try_build()?;
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

        runtime.block_on(future(client))
    }

    let arguments = Arguments::parse();
    let mut cache_manager = CACacheManager::default();

    if let Some(ref dir) = arguments.config.cache_dir {
        cache_manager.path = dir.to_path_buf();
    }

    match arguments.command {
        Command::SaveConfig { .. } => self::save_config(arguments),
        Command::Search { .. } => run(cache_manager, |c| self::search(arguments, c)),
    }
}
