use futures_lite::prelude::*;
use futures_lite::stream::once;
use zbus::proxy;
use zbus::zvariant;
use zbus::zvariant::OwnedValue;
use zbus::Connection;
use zbus::Error;
use zbus::Result;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemePreference {
    #[default]
    NoPreference = 0,
    Dark = 1,
    Light = 2,
}

impl TryFrom<zvariant::Value<'_>> for SchemePreference {
    type Error = Error;

    // https://github.com/flatpak/xdg-desktop-portal/blob/c0f0eb103effdcf3701a1bf53f12fe953fbf0b75/data/org.freedesktop.impl.portal.Settings.xml#L37
    fn try_from(value: zvariant::Value) -> Result<Self> {
        Ok(match u32::try_from(value)? {
            0 => SchemePreference::NoPreference,
            1 => SchemePreference::Dark,
            2 => SchemePreference::Light,
            _ => SchemePreference::NoPreference,
        })
    }
}

impl<'s> TryFrom<&'s zbus::Message> for SchemePreference {
    type Error = Error;
    fn try_from(message: &'s zbus::Message) -> Result<Self> {
        let body = message.body();
        let (_, _, arg): (&str, &str, zvariant::Value<'_>) = body.deserialize()?;
        SchemePreference::try_from(arg)
    }
}

#[derive(Debug, Clone)]
pub struct SchemeProxy<'a>(zbus::Proxy<'a>);

impl<'a> ::zbus::proxy::Defaults for SchemeProxy<'a> {
    const INTERFACE: &'static Option<zbus::names::InterfaceName<'static>> = &Some(
        zbus::names::InterfaceName::from_static_str_unchecked("org.freedesktop.portal.Settings"),
    );
    const DESTINATION: &'static Option<zbus::names::BusName<'static>> =
        &Some(zbus::names::BusName::WellKnown({
            zbus::names::WellKnownName::from_static_str_unchecked("org.freedesktop.portal.Desktop")
        }));
    const PATH: &'static Option<zbus::zvariant::ObjectPath<'static>> = &Some(
        zbus::zvariant::ObjectPath::from_static_str_unchecked("/org/freedesktop/portal/desktop"),
    );
}

impl<'c> From<zbus::Proxy<'c>> for SchemeProxy<'c> {
    fn from(proxy: zbus::Proxy<'c>) -> Self {
        SchemeProxy(proxy)
    }
}

impl<'a> SchemeProxy<'a> {
    pub async fn new(conn: &Connection) -> Result<SchemeProxy<'a>> {
        Self::builder(conn).build().await
    }

    pub async fn with_new_connection() -> Result<SchemeProxy<'a>> {
        Self::new(&zbus::Connection::session().await?).await
    }

    pub fn builder(conn: &::zbus::Connection) -> proxy::Builder<'a, Self> {
        proxy::Builder::new(conn).cache_properties(zbus::proxy::CacheProperties::No)
    }

    pub async fn read(&self) -> Result<SchemePreference> {
        let reply: OwnedValue = self.0.call("Read", &(NAMESPACE, KEY)).await?;
        reply.downcast_ref::<zvariant::Value>().map_or_else(
            |_| Err(zvariant::Error::IncorrectType.into()),
            SchemePreference::try_from,
        )
    }

    // Can contain duplicates
    async fn receive_changed(&self) -> Result<impl Stream<Item = SchemePreference>> {
        let signal = self
            .0
            .receive_signal_with_args("SettingChanged", &[(0, NAMESPACE), (1, KEY)])
            .await?
            .filter_map(|x| SchemePreference::try_from(&x).ok());

        Ok(signal)
    }

    pub async fn init_and_receive_changed(&self) -> Result<impl Stream<Item = SchemePreference>> {
        let mut preference = self.read().await?;
        Ok(
            once(preference).chain(self.receive_changed().await?.filter_map(move |p| {
                if p == preference {
                    None
                } else {
                    preference = p;
                    Some(p)
                }
            })),
        )
    }
}
