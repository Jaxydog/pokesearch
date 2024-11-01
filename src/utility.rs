use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use rustemon::Follow;
use rustemon::client::RustemonClient;
use rustemon::model::pokemon::{Type, TypeRelations};
use rustemon::model::resource::{Name, NamedApiResource};
use rustemon::model::utility::Language;

#[derive(Clone, Debug)]
pub struct TypeMatchup<'cl> {
    inner: HashMap<i64, (Arc<str>, f64)>,
    cache: Vec<(f64, Vec<Arc<str>>)>,
    client: &'cl RustemonClient,
}

#[allow(unused)]
impl<'cl> TypeMatchup<'cl> {
    pub async fn new(client: &'cl RustemonClient) -> Result<Self> {
        let mut this = Self { inner: HashMap::new(), cache: Vec::new(), client };

        for type_ in rustemon::pokemon::type_::get_all_entries(client).await? {
            let type_ = type_.follow(client).await?;

            if type_.id < 19 {
                let type_name = english_search(&type_.names)?.name.to_owned();

                this.inner.insert(type_.id, (type_name.into(), 1.0));
            }
        }

        Ok(this)
    }

    fn modify_type(&mut self, type_: &Type, modify: impl FnOnce(&mut f64)) {
        if !self.cache.is_empty() {
            self.cache.clear();
        }

        self.inner.entry(type_.id).and_modify(|(_, v)| modify(v));
    }

    pub async fn apply_relations(&mut self, relations: &TypeRelations) -> Result<()> {
        for type_ in &relations.no_damage_from {
            self.no_damage_from_resource(type_).await?;
        }
        for type_ in &relations.double_damage_from {
            self.double_damage_from_resource(type_).await?;
        }
        for type_ in &relations.half_damage_from {
            self.half_damage_from_resource(type_).await?;
        }

        Ok(())
    }

    pub fn no_damage_from(&mut self, type_: &Type) {
        self.modify_type(type_, |v| *v = 0.0);
    }

    pub fn half_damage_from(&mut self, type_: &Type) {
        self.modify_type(type_, |v| *v /= 2.0);
    }

    pub fn double_damage_from(&mut self, type_: &Type) {
        self.modify_type(type_, |v| *v *= 2.0);
    }

    pub async fn no_damage_from_name(&mut self, type_: &str) -> Result<()> {
        let type_ = rustemon::pokemon::type_::get_by_name(type_, self.client).await?;

        self.modify_type(&type_, |v| *v = 0.0);

        Ok(())
    }

    pub async fn half_damage_from_name(&mut self, type_: &str) -> Result<()> {
        let type_ = rustemon::pokemon::type_::get_by_name(type_, self.client).await?;

        self.modify_type(&type_, |v| *v /= 2.0);

        Ok(())
    }

    pub async fn double_damage_from_name(&mut self, type_: &str) -> Result<()> {
        let type_ = rustemon::pokemon::type_::get_by_name(type_, self.client).await?;

        self.modify_type(&type_, |v| *v *= 2.0);

        Ok(())
    }

    pub async fn no_damage_from_resource(&mut self, type_: &NamedApiResource<Type>) -> Result<()> {
        let type_ = type_.follow(self.client).await?;

        self.modify_type(&type_, |v| *v = 0.0);

        Ok(())
    }

    pub async fn half_damage_from_resource(&mut self, type_: &NamedApiResource<Type>) -> Result<()> {
        let type_ = type_.follow(self.client).await?;

        self.modify_type(&type_, |v| *v /= 2.0);

        Ok(())
    }

    pub async fn double_damage_from_resource(&mut self, type_: &NamedApiResource<Type>) -> Result<()> {
        let type_ = type_.follow(self.client).await?;

        self.modify_type(&type_, |v| *v *= 2.0);

        Ok(())
    }

    pub fn get(&mut self) -> impl Iterator<Item = (f64, &[Arc<str>])> {
        if self.cache.is_empty() {
            self.cache = self
                .inner
                .iter()
                .fold(HashMap::<u16, Vec<Arc<str>>>::new(), |mut map, (_, (name, mult))| {
                    map.entry((*mult * 100.0).round() as u16).or_default().push(Arc::clone(name));

                    map
                })
                .into_iter()
                .map(|(m, mut v)| {
                    v.dedup();
                    v.sort_unstable();

                    (m as f64 / 100.0, v)
                })
                .collect::<Vec<_>>();

            self.cache.sort_unstable_by_key(|(m, _)| (*m * 100.0) as u16);
            self.cache.reverse();
        }

        self.cache.iter().map(|(mult, list)| (*mult, &**list))
    }

    pub async fn print(&mut self) -> Result<()> {
        for (multiplier, type_list) in self.get() {
            crate::async_println!("×{multiplier}\t{}", type_list.join(", ")).await?;
        }

        Ok(())
    }
}

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
