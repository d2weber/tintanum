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
