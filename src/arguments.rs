use clap::{Parser, ValueEnum};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
#[command(about, author, version, long_about = None)]
pub struct Arguments {
    /// The search type.
    pub kind: SearchKind,
    /// The search text.
    pub text: Box<str>,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SearchKind {
    Pokemon,
    Ability,
    Move,
    Item,
}
