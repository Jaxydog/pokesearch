use anyhow::{Result, bail};
use rustemon::model::resource::{Name, NamedApiResource};
use rustemon::model::utility::Language;

#[macro_export]
macro_rules! async_print {
    ($($args:tt)+) => {
        <_ as ::tokio::io::AsyncWriteExt>::write_all(&mut ::tokio::io::stdout(), ::std::format!($($args)+).as_bytes())
    };
}

#[macro_export]
macro_rules! async_println {
    ($($args:tt)+) => {
        async {
            let mut stdout = ::tokio::io::stdout();

            <_ as ::tokio::io::AsyncWriteExt>::write_all(&mut stdout, ::std::format!($($args)+).as_bytes()).await?;
            <_ as ::tokio::io::AsyncWriteExt>::write_u8(&mut stdout, b'\n').await
        }
    };
}

#[inline]
pub fn linear_search<T>(list: &[T], predicate: impl Fn(&&T) -> bool) -> Result<&T> {
    match list.iter().find(predicate).or_else(|| list.first()) {
        Some(value) => Ok(value),
        None => bail!("unable to find a suitable value"),
    }
}

#[inline]
pub fn english_search(list: &[Name]) -> Result<&Name> {
    self::linear_search(list, |v| v.language.name == "en")
}

#[inline]
pub fn english_search_by<T>(list: &[T], get_name: impl Fn(&T) -> &NamedApiResource<Language>) -> Result<&T> {
    self::linear_search(list, |v| get_name(v).name == "en")
}
