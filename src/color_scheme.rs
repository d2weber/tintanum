use smol::prelude::*;
use smol::stream::once;
use zbus::dbus_proxy;
use zbus::Connection;

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

    fn try_from(value: zbus::zvariant::Value) -> zbus::Result<SchemePreference> {
        Ok(match u32::try_from(value)? {
            0 => SchemePreference::NoPreference,
            1 => SchemePreference::Dark,
            2 => SchemePreference::Light,
            _ => SchemePreference::NoPreference,
        })
    }
}

#[dbus_proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    gen_blocking = false
)]
trait Settings {
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<zbus::zvariant::OwnedValue>;

    #[dbus_proxy(signal)]
    fn setting_changed(
        &self,
        namespace: &str,
        key: &str,
        value: zbus::zvariant::Value<'_>,
    ) -> zbus::Result<()>;
}

pub struct SchemeProxy<'a> {
    proxy: SettingsProxy<'a>,
}

impl<'a> SchemeProxy<'a> {
    pub async fn new() -> zbus::Result<SchemeProxy<'a>> {
        let connection = Connection::session().await?;
        let proxy = SettingsProxy::new(&connection).await?;
        Ok(SchemeProxy { proxy })
    }

    pub async fn read(&self) -> zbus::Result<SchemePreference> {
        let v = self.proxy.read(NAMESPACE, KEY).await?;
        let v = v.downcast_ref::<zbus::zvariant::Value>().unwrap().clone();
        SchemePreference::try_from(v)
    }

    pub async fn receive_changed(&self) -> zbus::Result<impl Stream<Item = SchemePreference>> {
        let mut preference = self.read().await?;
        let signal = self
            .proxy
            .receive_setting_changed_with_args(&[(0, NAMESPACE), (1, KEY)])
            .await?;
        Ok(once(preference).chain(signal.filter_map(move |x| {
            SchemePreference::try_from(x.args().ok()?.value)
                .ok()
                .and_then(|p| {
                    if p == preference {
                        return None;
                    }
                    preference = p;
                    Some(p)
                })
        })))
    }
}
