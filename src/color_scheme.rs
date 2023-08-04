use smol::prelude::*;
use smol::stream::once;
use zbus::zvariant::OwnedValue;
use zbus::Connection;
use zbus::ProxyBuilder;

const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";

// https://github.com/flatpak/xdg-desktop-portal/blob/c0f0eb103effdcf3701a1bf53f12fe953fbf0b75/data/org.freedesktop.impl.portal.Settings.xml#L37
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SchemePreference {
    NoPreference = 0,
    Dark = 1,
    Light = 2,
}

impl TryFrom<zbus::zvariant::Value<'_>> for SchemePreference {
    type Error = zbus::Error;

    fn try_from(value: zbus::zvariant::Value) -> zbus::Result<Self> {
        Ok(match u32::try_from(value)? {
            0 => SchemePreference::NoPreference,
            1 => SchemePreference::Dark,
            2 => SchemePreference::Light,
            _ => SchemePreference::NoPreference,
        })
    }
}

impl<'s> TryFrom<&'s ::zbus::Message> for SchemePreference {
    type Error = ::zbus::Error;
    fn try_from(message: &'s ::zbus::Message) -> ::zbus::Result<Self> {
        message
            .body::<(&str, &str, zbus::zvariant::Value<'_>)>()
            .map_err(::std::convert::Into::into)
            .and_then(|args| SchemePreference::try_from(args.2))
    }
}

pub struct SchemeProxy<'a>(zbus::Proxy<'a>);

impl<'a> SchemeProxy<'a> {
    pub async fn new() -> zbus::Result<SchemeProxy<'a>> {
        let connection = Connection::session().await?;
        let proxy = ProxyBuilder::new_bare(&connection)
            .interface("org.freedesktop.portal.Settings")?
            .path("/org/freedesktop/portal/desktop")?
            .destination("org.freedesktop.portal.Desktop")?
            .build()
            .await?;
        Ok(Self(proxy))
    }

    pub async fn read(&self) -> zbus::Result<SchemePreference> {
        let reply: OwnedValue = self.0.call("Read", &(NAMESPACE, KEY)).await?;
        reply
            .downcast_ref::<zbus::zvariant::Value>()
            .cloned()
            .ok_or(zbus::zvariant::Error::IncorrectType.into())
            .and_then(SchemePreference::try_from)
    }

    // Can contain duplicates
    async fn receive_changed(&self) -> zbus::Result<impl Stream<Item = SchemePreference>> {
        let signal = self
            .0
            .receive_signal_with_args("SettingChanged", &[(0, NAMESPACE), (1, KEY)])
            .await?
            .filter_map(|x| SchemePreference::try_from(&*x).ok());

        Ok(signal)
    }

    pub async fn init_and_receive_changed(
        &self,
    ) -> zbus::Result<impl Stream<Item = SchemePreference>> {
        let mut preference = self.read().await?;
        Ok(
            once(preference).chain(self.receive_changed().await?.filter_map(move |p| {
                if p == preference {
                    return None;
                }
                preference = p;
                Some(p)
            })),
        )
    }
}
