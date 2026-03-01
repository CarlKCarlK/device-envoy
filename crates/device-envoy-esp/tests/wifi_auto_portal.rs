use device_envoy_esp32::wifi_auto::fields::{TextField, TimezoneField};
use device_envoy_esp32::wifi_auto::{WifiAuto, WifiAutoField, WifiCredentials, WifiStartMode};

#[test]
fn parse_post_decodes_credentials() {
    let wifi_auto = WifiAuto::new("PortalSsid", &[]);
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\nContent-Length: 29\r\n\r\nssid=Home+WiFi&password=s3cr%21";
    let wifi_credentials = wifi_auto
        .parse_post(request)
        .expect("valid credentials expected");

    assert_eq!(wifi_credentials.ssid.as_str(), "Home WiFi");
    assert_eq!(wifi_credentials.password.as_str(), "s3cr!");
}

#[test]
fn parse_post_applies_custom_field_parsing() {
    let timezone_field_static = Box::leak(Box::new(TimezoneField::new_static()));
    let timezone_field = TimezoneField::new(timezone_field_static);
    let fields: [&'static dyn WifiAutoField; 1] = [timezone_field];
    let wifi_auto = WifiAuto::new("PortalSsid", &fields);

    let request =
        "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&password=abc123&timezone=-300";
    let _wifi_credentials = wifi_auto
        .parse_post(request)
        .expect("valid request expected");

    assert_eq!(
        timezone_field
            .offset_minutes()
            .expect("timezone offset should load"),
        Some(-300)
    );
    assert!(wifi_auto
        .custom_fields_satisfied()
        .expect("state query should succeed"));
}

#[test]
fn parse_post_requires_ssid() {
    let wifi_auto = WifiAuto::new("PortalSsid", &[]);
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\npassword=s3cr3t";

    assert!(wifi_auto.parse_post(request).is_none());
}

#[test]
fn generate_config_page_escapes_defaults() {
    let wifi_auto = WifiAuto::new("PortalSsid", &[]);
    let wifi_credentials =
        device_envoy_esp32::wifi_auto::WifiCredentials::new("A&B\"<ssid>", "p@ss<word>&\"");

    let page = wifi_auto.generate_config_page(Some(&wifi_credentials));

    assert!(page.contains("A&amp;B&quot;&lt;ssid&gt;"));
    assert!(page.contains("p@ss&lt;word&gt;&amp;&quot;"));
}

#[test]
fn text_field_roundtrip_from_post() {
    let text_field_static = Box::leak(Box::new(TextField::<16>::new_static()));
    let text_field = TextField::new(
        text_field_static,
        "device_name",
        "Device name",
        "DeskSensor",
    );
    let fields: [&'static dyn WifiAutoField; 1] = [text_field];
    let wifi_auto = WifiAuto::new("PortalSsid", &fields);

    let request =
        "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Lab&password=abc123&device_name=Panel01";
    let _wifi_credentials = wifi_auto
        .parse_post(request)
        .expect("valid request expected");

    assert_eq!(
        text_field
            .text()
            .expect("text should load")
            .expect("text should be present")
            .as_str(),
        "Panel01"
    );
    assert!(text_field
        .is_satisfied()
        .expect("state query should succeed"));
}

#[test]
fn wifi_auto_credentials_roundtrip_in_memory() {
    let wifi_auto = WifiAuto::new("PortalSsid", &[]);
    let wifi_credentials = WifiCredentials::new("OfficeWifi", "supersecret");

    assert!(wifi_auto
        .load_persisted_credentials()
        .expect("credentials load should succeed")
        .is_none());

    wifi_auto
        .persist_credentials(&wifi_credentials)
        .expect("credentials save should succeed");

    let loaded_wifi_credentials = wifi_auto
        .load_persisted_credentials()
        .expect("credentials load should succeed")
        .expect("credentials should exist");
    assert_eq!(loaded_wifi_credentials, wifi_credentials);

    wifi_auto
        .clear_persisted_credentials()
        .expect("credentials clear should succeed");
    assert!(wifi_auto
        .load_persisted_credentials()
        .expect("credentials load should succeed")
        .is_none());
}

#[test]
fn wifi_auto_start_mode_roundtrip_in_memory() {
    let wifi_auto = WifiAuto::new("PortalSsid", &[]);

    assert_eq!(
        wifi_auto
            .start_mode()
            .expect("start mode load should succeed"),
        WifiStartMode::Client
    );

    wifi_auto
        .set_start_mode(WifiStartMode::CaptivePortal)
        .expect("start mode save should succeed");

    assert_eq!(
        wifi_auto
            .start_mode()
            .expect("start mode load should succeed"),
        WifiStartMode::CaptivePortal
    );
}
