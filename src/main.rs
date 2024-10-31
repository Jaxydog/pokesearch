use std::collections::HashMap;
use std::error::Error;
use std::future::Future;

use anyhow::{Result, bail};
use arguments::{Arguments, SearchKind};
use clap::Parser;
use rustemon::Follow;
use rustemon::client::{CACacheManager, RustemonClient, RustemonClientBuilder};
use utility::{english_search, english_search_by};

mod arguments;
mod utility;

fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let manager = CACacheManager { path: (&*arguments.cache_dir).into() };
    let client = RustemonClientBuilder::default().with_manager(manager).try_build()?;
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(self::async_main(&arguments, client))
}

async fn async_main(arguments: &Arguments, client: RustemonClient) -> Result<()> {
    let api_text = arguments.text.replace(' ', "-").to_lowercase();

    match arguments.kind {
        SearchKind::Pokemon => self::run_pokemon(arguments, client, &api_text).await,
        SearchKind::Ability => self::run_ability(arguments, client, &api_text).await,
        SearchKind::Move => self::run_move(arguments, client, &api_text).await,
        SearchKind::Item => self::run_item(arguments, client, &api_text).await,
    }
}

#[inline]
async fn search<T, E: Error>(name: &'static str, text: &str, future: impl Future<Output = Result<T, E>>) -> Result<T> {
    match future.await {
        Ok(value) => Ok(value),
        Err(error) => bail!("failed to resolve {name} '{text}' - {error}"),
    }
}

async fn run_pokemon(arguments: &Arguments, client: RustemonClient, api_text: &str) -> Result<()> {
    let pokemon =
        self::search("pokemon", &arguments.text, rustemon::pokemon::pokemon::get_by_name(api_text, &client)).await?;

    let species = pokemon.species.follow(&client).await?;
    let species_name = &english_search(&species.names)?.name;
    let species_generation = english_search(&species.generation.follow(&client).await?.names)?.name.to_owned();

    async_println!("{species_name} ({species_generation})\n").await?;

    let mut pokemon_types = pokemon.types.clone();
    let mut pokemon_type_names = Vec::with_capacity(pokemon_types.len());

    pokemon_types.sort_unstable_by_key(|v| v.slot);

    let mut type_matchup = HashMap::<i64, (String, f64)>::new();

    for type_ in rustemon::pokemon::type_::get_all_entries(&client).await? {
        let type_ = type_.follow(&client).await?;
        let type_name = english_search(&type_.names)?.name.to_owned();

        type_matchup.insert(type_.id, (type_name, 1.0));
    }

    for type_ in &pokemon_types {
        let type_ = type_.type_.follow(&client).await?;

        pokemon_type_names.push(english_search(&type_.names)?.name.to_owned());

        for type_ in type_.damage_relations.no_damage_from {
            let type_ = type_.follow(&client).await?;

            type_matchup.entry(type_.id).and_modify(|(_, v)| *v = 0.0);
        }
        for type_ in type_.damage_relations.double_damage_from {
            let type_ = type_.follow(&client).await?;

            type_matchup.entry(type_.id).and_modify(|(_, v)| *v *= 2.0);
        }
        for type_ in type_.damage_relations.half_damage_from {
            let type_ = type_.follow(&client).await?;

            type_matchup.entry(type_.id).and_modify(|(_, v)| *v /= 2.0);
        }
    }

    async_println!("Types:\t{}", pokemon_type_names.join(", ")).await?;

    let pokemon_weight = pokemon.weight as f64 / 10.0;

    async_println!("Weight:\t{pokemon_weight} kg\n").await?;

    let type_matchup = type_matchup.iter().fold(HashMap::<u16, Vec<&str>>::new(), |mut map, (_, (name, mult))| {
        let multiplier = (*mult * 100.0).round() as u16;

        map.entry(multiplier).or_default().push(name);

        map
    });

    let mut type_matchup = type_matchup.into_iter().collect::<Box<[_]>>();

    type_matchup.sort_unstable_by_key(|v| v.0);
    type_matchup.reverse();

    for (multiplier, mut type_list) in type_matchup {
        type_list.dedup();
        type_list.sort_unstable();

        let multiplier = multiplier as f64 / 100.0;

        async_println!("×{multiplier}\t{}", type_list.join(", ")).await?;
    }

    Ok(())
}

async fn run_ability(arguments: &Arguments, client: RustemonClient, api_text: &str) -> Result<()> {
    let ability =
        self::search("ability", &arguments.text, rustemon::pokemon::ability::get_by_name(api_text, &client)).await?;

    let ability_name = &english_search(&ability.names)?.name;
    let ability_generation = english_search(&ability.generation.follow(&client).await?.names)?.name.to_owned();
    let ability_effect = &english_search_by(&ability.effect_entries, |v| &v.language)?.effect;

    async_println!("{ability_name} ({ability_generation})\n\n---\n\n{ability_effect}").await.map_err(Into::into)
}

async fn run_move(arguments: &Arguments, client: RustemonClient, api_text: &str) -> Result<()> {
    let move_ = self::search("move", &arguments.text, rustemon::moves::move_::get_by_name(api_text, &client)).await?;

    let move_name = &english_search(&move_.names)?.name;
    let move_generation = english_search(&move_.generation.follow(&client).await?.names)?.name.to_owned();

    async_println!("{move_name} ({move_generation})\n").await?;

    let move_class = english_search(&move_.damage_class.follow(&client).await?.names)?.name.to_owned();
    let move_class = move_class.chars().take(1).map(|c| c.to_ascii_uppercase()).chain(move_class.chars().skip(1));

    async_println!("Class:\t\t{}", move_class.collect::<Box<str>>()).await?;

    let move_type = english_search(&move_.type_.follow(&client).await?.names)?.name.to_owned();

    async_println!("Type:\t\t{move_type}").await?;

    if let Some(move_pp) = move_.pp {
        async_println!("PP:\t\t{move_pp}").await?;
    } else {
        async_println!("PP:\t\t-").await?;
    }

    if let Some(move_power) = move_.power {
        async_println!("Power:\t\t{move_power}").await?;
    } else {
        async_println!("Power:\t\t-").await?;
    }

    if let Some(move_accuracy) = move_.accuracy {
        async_println!("Accuracy:\t{move_accuracy}").await?;
    } else {
        async_println!("Accuracy:\t-").await?;
    }

    if move_.priority != 0 {
        async_println!("Priority:\t{}", move_.priority).await?;
    }

    let move_target = english_search(&move_.target.follow(&client).await?.names)?.name.to_owned();
    let move_effect = &english_search_by(&move_.effect_entries, |v| &v.language)?.effect;

    async_println!("Target: {move_target}\n\n---\n\n{move_effect}").await.map_err(Into::into)
}

async fn run_item(arguments: &Arguments, client: RustemonClient, api_text: &str) -> Result<()> {
    let item = self::search("item", &arguments.text, rustemon::items::item::get_by_name(api_text, &client)).await?;

    let item_name = &english_search(&item.names)?.name;
    let item_category = english_search(&item.category.follow(&client).await?.names)?.name.to_owned();

    async_println!("{item_name} ({item_category})\n\n---\n").await?;

    if let Some((item_fling_effect, item_fling_power)) = item.fling_effect.zip(item.fling_power) {
        let item_fling_effect = item_fling_effect.follow(&client).await?.effect_entries;
        let item_fling_effect = &english_search_by(&item_fling_effect, |v| &v.language)?.effect;

        async_println!("Thrown with fling ({item_fling_power} power)\n:   {item_fling_effect}\n").await?;
    }

    let item_effect = &english_search_by(&item.effect_entries, |v| &v.language)?.effect;

    async_println!("{item_effect}").await.map_err(Into::into)
}
