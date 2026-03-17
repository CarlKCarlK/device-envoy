use device_envoy_core::wifi_auto::{generate_config_page, parse_post, WifiCredentials};
use device_envoy_esp::wifi_auto::{
    WifiAutoField,
    fields::{TextField, TimezoneField},
};

#[test]
fn parse_post_decodes_credentials() {
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\nContent-Length: 29\r\n\r\nssid=Home+WiFi&password=s3cr%21";
    let wifi_credentials = parse_post::<device_envoy_esp::Error>(request, None, &[])
        .expect("valid credentials expected");

    assert_eq!(wifi_credentials.ssid.as_str(), "Home WiFi");
    assert_eq!(wifi_credentials.password.as_str(), "s3cr!");
}

#[test]
fn parse_post_applies_custom_field_parsing() {
    let timezone_field_static = Box::leak(Box::new(TimezoneField::new_static()));
    let timezone_field = TimezoneField::new(timezone_field_static);
    let fields: [&'static dyn WifiAutoField<Error = device_envoy_esp::Error>; 1] = [timezone_field];

    let request =
        "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&password=abc123&timezone=-300";
    let _wifi_credentials = parse_post::<device_envoy_esp::Error>(request, None, &fields)
        .expect("valid request expected");

    assert_eq!(
        timezone_field
            .offset_minutes()
            .expect("timezone offset should load"),
        Some(-300)
    );
    assert!(timezone_field
        .is_satisfied()
        .expect("state query should succeed"));
}

#[test]
fn parse_post_requires_ssid() {
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\npassword=s3cr3t";

    assert!(parse_post::<device_envoy_esp::Error>(request, None, &[]).is_none());
}

#[test]
fn generate_config_page_escapes_defaults() {
    let wifi_credentials = WifiCredentials::new("A&B\"<ssid>", "p@ss<word>&\"");

    let page = generate_config_page::<device_envoy_esp::Error>(Some(&wifi_credentials), &[]);

    assert!(page.contains("A&amp;B&quot;&lt;ssid&gt;"));
    assert!(!page.contains("p@ss&lt;word&gt;&amp;&quot;"));
    assert!(page.contains("name=\"password\""));
    assert!(page.contains("name=\"keep_saved_password\""));
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
    let fields: [&'static dyn WifiAutoField<Error = device_envoy_esp::Error>; 1] = [text_field];

    let request =
        "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Lab&password=abc123&device_name=Panel01";
    let _wifi_credentials = parse_post::<device_envoy_esp::Error>(request, None, &fields)
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
fn parse_post_keeps_saved_password_when_checkbox_selected() {
    let defaults_wifi_credentials = WifiCredentials::new("Office", "saved-secret");
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&keep_saved_password=1";

    let wifi_credentials =
        parse_post::<device_envoy_esp::Error>(request, Some(&defaults_wifi_credentials), &[])
            .expect("valid credentials expected");

    assert_eq!(wifi_credentials.ssid.as_str(), "Office");
    assert_eq!(wifi_credentials.password.as_str(), "saved-secret");
}

#[test]
fn parse_post_rejects_keep_saved_password_without_defaults() {
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&keep_saved_password=1";

    assert!(parse_post::<device_envoy_esp::Error>(request, None, &[]).is_none());
}

#[test]
fn generate_config_page_hides_keep_saved_password_when_saved_password_is_blank() {
    let wifi_credentials = WifiCredentials::new("Office", "");

    let page = generate_config_page::<device_envoy_esp::Error>(Some(&wifi_credentials), &[]);

    assert!(!page.contains("name=\"keep_saved_password\""));
}

#[test]
fn parse_post_rejects_keep_saved_password_when_saved_password_is_blank() {
    let defaults_wifi_credentials = WifiCredentials::new("Office", "");
    let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&keep_saved_password=1";

    assert!(
        parse_post::<device_envoy_esp::Error>(request, Some(&defaults_wifi_credentials), &[])
            .is_none()
    );
}
