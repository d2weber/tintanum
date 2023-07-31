use zbus::dbus_proxy;

#[dbus_proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    assume_defaults = true
)]
trait Settings {
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<zbus::zvariant::OwnedValue>;

    fn read_all(
        &self,
        namespaces: &[&str],
    ) -> zbus::Result<
        std::collections::HashMap<
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        >,
    >;

    #[dbus_proxy(signal)]
    fn setting_changed(
        &self,
        namespace: &str,
        key: &str,
        value: zbus::zvariant::Value<'_>,
    ) -> zbus::Result<()>;

    #[dbus_proxy(property, name = "version")]
    fn version(&self) -> zbus::Result<u32>;
}
