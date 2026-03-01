use core::fmt::Write;

use heapless::{FnvIndexMap, String};

use super::WifiCredentials;
use crate::Result;

pub type HtmlBuffer = String<16384>;

/// Trait for custom setup fields rendered and parsed by the captive portal.
pub trait WifiAutoField {
    /// Render form HTML elements.
    fn render(&self, page: &mut HtmlBuffer) -> Result<()>;

    /// Parse submitted form data.
    fn parse(&self, form: &FormData<'_>) -> Result<()>;

    /// Whether this field currently has valid configured data.
    fn is_satisfied(&self) -> Result<bool> {
        Ok(true)
    }
}

pub struct FormData<'a> {
    params: &'a FormMap,
}

impl<'a> FormData<'a> {
    fn new(params: &'a FormMap) -> Self {
        Self { params }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(stored_key, _)| stored_key.as_str() == key)
            .map(|(_, value)| value.as_str())
    }
}

type FormKey = String<32>;
type FormValue = String<256>;
type FormMap = FnvIndexMap<FormKey, FormValue, 32>;

pub(super) fn parse_post(request: &str, fields: &[&dyn WifiAutoField]) -> Option<WifiCredentials> {
    let body_start = request.find("\r\n\r\n")? + 4;
    let body = &request[body_start..];

    let mut params: FormMap = FormMap::new();
    let mut ssid = heapless::String::<32>::new();
    let mut password = heapless::String::<64>::new();

    for param in body.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            let decoded_key = url_decode::<32>(key);
            let decoded_value = url_decode::<256>(value);
            params
                .insert(decoded_key.clone(), decoded_value.clone())
                .ok()?;
            match decoded_key.as_str() {
                "ssid" => {
                    ssid.push_str(&decoded_value).ok()?;
                }
                "password" => {
                    password.push_str(&decoded_value).ok()?;
                }
                _ => {}
            }
        }
    }

    if ssid.is_empty() {
        return None;
    }

    let form_data = FormData::new(&params);
    for field in fields {
        field.parse(&form_data).ok()?;
    }

    Some(WifiCredentials { ssid, password })
}

pub(super) fn generate_config_page(
    defaults: Option<&WifiCredentials>,
    fields: &[&dyn WifiAutoField],
) -> HtmlBuffer {
    let mut page = HtmlBuffer::new();
    let ssid = defaults
        .as_ref()
        .map(|wifi_credentials| escape_html::<160>(wifi_credentials.ssid.as_str()))
        .unwrap_or_else(heapless::String::new);
    let password = defaults
        .as_ref()
        .map(|wifi_credentials| escape_html::<320>(wifi_credentials.password.as_str()))
        .unwrap_or_else(heapless::String::new);

    write!(
        page,
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html\r\n\
         Connection: close\r\n\
         \r\n\
         <!DOCTYPE html>\
         <html>\
         <head>\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
             <title>WiFi Configuration</title>\
             <link rel=\"icon\" href=\"data:,\">\
             <style>\
                 body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; }}\
                 h1 {{ color: #333; }}\
                 form {{ margin-top: 20px; }}\
                 input, select {{ width: 100%; padding: 10px; margin: 10px 0; box-sizing: border-box; }}\
                 label {{ display: block; margin-top: 10px; }}\
                 .toggle {{ display: flex; align-items: center; gap: 8px; font-size: 0.9rem; color: #444; margin-top: 5px; }}\
                 .toggle input {{ width: auto; margin: 0; }}\
                 button {{ width: 100%; padding: 12px; background-color: #2e7d32; color: white; border: none; cursor: pointer; }}\
                 button:hover {{ background-color: #1b5e20; }}\
             </style>\
             <script>\
                 function togglePasswordVisibility() {{\
                     var input = document.getElementById('password');\
                     input.type = input.type === 'password' ? 'text' : 'password';\
                 }}\
             </script>\
         </head>\
         <body>\
             <h1>WiFi Configuration</h1>\
             <p>Enter your WiFi network credentials:</p>\
             <form method=\"POST\" action=\"/\">\
                <label for=\"ssid\">WiFi Network Name (SSID):</label>\
                <input type=\"text\" id=\"ssid\" name=\"ssid\" value=\"{}\" required>\
                <label for=\"password\">Password:</label>\
                <input type=\"password\" id=\"password\" name=\"password\" value=\"{}\" required>\
                <label class=\"toggle\"><input type=\"checkbox\" onclick=\"togglePasswordVisibility()\">Show password</label>\
",
        ssid, password
    )
    .expect("page HTML exceeds capacity");

    for field in fields {
        field
            .render(&mut page)
            .expect("custom field HTML exceeds page capacity");
    }

    page.push_str("<button type=\"submit\">Connect</button></form></body></html>")
        .expect("page HTML exceeds capacity");

    page
}

fn url_decode<const N: usize>(input: &str) -> heapless::String<N> {
    let mut output = heapless::String::<N>::new();
    let mut chars = input.chars();

    while let Some(character) = chars.next() {
        if character == '+' {
            output.push(' ').expect("decoded URL exceeds capacity");
        } else if character == '%' {
            if let (Some(high), Some(low)) = (chars.next(), chars.next()) {
                if let (Some(high_digit), Some(low_digit)) = (high.to_digit(16), low.to_digit(16)) {
                    #[allow(clippy::cast_possible_truncation)]
                    let byte = ((high_digit << 4) | low_digit) as u8;
                    if let Ok(decoded) = core::str::from_utf8(&[byte]) {
                        output
                            .push_str(decoded)
                            .expect("decoded URL exceeds capacity");
                    }
                }
            }
        } else {
            output
                .push(character)
                .expect("decoded URL exceeds capacity");
        }
    }

    output
}

fn escape_html<const N: usize>(value: &str) -> heapless::String<N> {
    let mut escaped = heapless::String::<N>::new();
    for character in value.chars() {
        match character {
            '&' => escaped
                .push_str("&amp;")
                .expect("escaped HTML exceeds capacity"),
            '<' => escaped
                .push_str("&lt;")
                .expect("escaped HTML exceeds capacity"),
            '>' => escaped
                .push_str("&gt;")
                .expect("escaped HTML exceeds capacity"),
            '"' => escaped
                .push_str("&quot;")
                .expect("escaped HTML exceeds capacity"),
            '\'' => escaped
                .push_str("&#39;")
                .expect("escaped HTML exceeds capacity"),
            _ => escaped
                .push(character)
                .expect("escaped HTML exceeds capacity"),
        }
    }
    escaped
}
