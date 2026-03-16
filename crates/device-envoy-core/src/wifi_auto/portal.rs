use core::fmt::Write;

use heapless::{FnvIndexMap, String};

use super::WifiCredentials;

/// Captive-portal page buffer type.
pub type HtmlBuffer = String<16384>;

/// Shared trait for custom setup fields rendered and parsed by the captive portal.
///
/// This is the platform-agnostic contract used by both RP and ESP ports.
///
/// To define your own field type, implement:
///
/// - [`WifiAutoField::render`] to write the HTML form UI.
/// - [`WifiAutoField::parse`] to read submitted form values and typically persist them.
/// - [`WifiAutoField::is_satisfied`] when setup should require the field.
///
/// For ready-to-use field types, see platform modules:
/// `device_envoy_rp::wifi_auto::fields` and `device_envoy_esp::wifi_auto::fields`.
///
/// # Example
///
/// This example mirrors the platform example `wifi_auto_custom_checkbox.rs`, but keeps
/// everything core-only. It shows a checkbox field that:
///
/// - renders a checkbox in the captive portal,
/// - parses submitted form data,
/// - stores the latest value in field-local state, and
/// - requires at least one explicit user choice before setup is considered complete.
///
/// ```rust,no_run
/// # #![no_std]
/// use core::{cell::Cell, convert::Infallible, fmt::Write as _};
///
/// use device_envoy_core::wifi_auto::{
///     FormData, HtmlBuffer, WifiAutoField, parse_post,
/// };
///
/// // Custom field type used by WifiAuto setup.
/// struct CheckboxField {
///     // HTML form key.
///     field_name: &'static str,
///     // Label shown next to the checkbox.
///     label: &'static str,
///     // Render-only fallback before the first submit.
///     default_checked: bool,
///     // Tri-state: None = never configured, Some(true/false) = explicit user choice.
///     checked: Cell<Option<bool>>,
/// }
///
/// impl CheckboxField {
///     const fn new(
///         field_name: &'static str,
///         label: &'static str,
///         default_checked: bool,
///     ) -> Self {
///         Self {
///             field_name,
///             label,
///             default_checked,
///             checked: Cell::new(None),
///         }
///     }
///
///     fn checked(&self) -> Option<bool> {
///         self.checked.get()
///     }
///
///     // UI fallback only; setup completeness is driven by `checked()`.
///     fn checked_or_default(&self) -> bool {
///         self.checked().unwrap_or(self.default_checked)
///     }
/// }
///
/// impl WifiAutoField for CheckboxField {
///     type Error = Infallible;
///
///     fn render(&self, page: &mut HtmlBuffer) -> Result<(), Self::Error> {
///         // render() controls what appears on the captive-portal page.
///         let checked_attribute = if self.checked_or_default() {
///             " checked"
///         } else {
///             ""
///         };
///         let _ = write!(
///             page,
///             "<p><label><input type=\"checkbox\" name=\"{}\" value=\"1\"{}> {}</label></p>",
///             self.field_name, checked_attribute, self.label
///         );
///         Ok(())
///     }
///
///     fn parse(&self, form: &FormData<'_>) -> Result<(), Self::Error> {
///         // parse() reads the submitted field value.
///         let checked = matches!(
///             form.get(self.field_name),
///             Some("1") | Some("on") | Some("true")
///         );
///         // This example persists to in-memory field state; platform fields can
///         // persist to flash instead.
///         self.checked.set(Some(checked));
///         Ok(())
///     }
///
///     fn is_satisfied(&self) -> Result<bool, Self::Error> {
///         // Require at least one explicit submit before provisioning is complete.
///         Ok(self.checked().is_some())
///     }
/// }
///
/// fn example() {
///     // Instantiate the custom field.
///     let checkbox_field = CheckboxField::new(
///         "share_telemetry",
///         "Share anonymous telemetry",
///         false,
///     );
///     // Pass it to the same parse pipeline used by WifiAuto internals.
///     let fields: [&dyn WifiAutoField<Error = Infallible>; 1] = [&checkbox_field];
///
///     // Simulate a captive-portal form submission.
///     let request = "POST / HTTP/1.1\r\nHost: 192.168.4.1\r\n\r\nssid=Office&password=abc123&share_telemetry=1";
///     let _wifi_credentials = parse_post::<Infallible>(request, None, &fields);
/// }
/// # fn main() {
/// #     example();
/// # }
/// ```
pub trait WifiAutoField {
    /// Platform crate error type used by field implementations.
    type Error;

    /// Render form HTML elements.
    fn render(&self, page: &mut HtmlBuffer) -> Result<(), Self::Error>;

    /// Parse submitted form data and typically persist the field value.
    fn parse(&self, form: &FormData<'_>) -> Result<(), Self::Error>;

    /// Whether this field currently has valid configured data.
    fn is_satisfied(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Parsed key/value form data from a URL-encoded POST body.
#[doc(hidden)] // Shared parser view type used by platform/custom field plumbing.
pub struct FormData<'a> {
    params: &'a FormMap,
}

impl<'a> FormData<'a> {
    fn new(params: &'a FormMap) -> Self {
        Self { params }
    }

    /// Return the value for `key`.
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

/// Parse a raw HTTP POST request into Wi-Fi credentials and apply field parsers.
///
/// Returns `None` when the request body is malformed, the SSID is missing, or any custom
/// field parser reports an error.
#[must_use]
pub fn parse_post<E>(
    request: &str,
    defaults: Option<&WifiCredentials>,
    fields: &[&dyn WifiAutoField<Error = E>],
) -> Option<WifiCredentials> {
    let body_start = request.find("\r\n\r\n")? + 4;
    let body = &request[body_start..];

    let mut params: FormMap = FormMap::new();
    let mut ssid = heapless::String::<32>::new();
    let mut submitted_password = heapless::String::<64>::new();
    let mut keep_saved_password = false;

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
                    submitted_password.push_str(&decoded_value).ok()?;
                }
                "keep_saved_password" => {
                    keep_saved_password = decoded_value.as_str() == "1";
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

    let password = if keep_saved_password {
        let defaults_wifi_credentials = defaults?;
        if defaults_wifi_credentials.password.is_empty() {
            return None;
        }
        defaults_wifi_credentials.password.clone()
    } else {
        if submitted_password.is_empty() {
            return None;
        }
        submitted_password
    };

    Some(WifiCredentials { ssid, password })
}

/// Build the captive-portal configuration HTML page.
#[must_use]
pub fn generate_config_page<E>(
    defaults: Option<&WifiCredentials>,
    fields: &[&dyn WifiAutoField<Error = E>],
) -> HtmlBuffer {
    let mut page = HtmlBuffer::new();
    let ssid = defaults
        .as_ref()
        .map(|wifi_credentials| escape_html::<160>(wifi_credentials.ssid.as_str()))
        .unwrap_or_else(heapless::String::new);
    let has_saved_password = defaults
        .as_ref()
        .map(|wifi_credentials| !wifi_credentials.password.is_empty())
        .unwrap_or(false);
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
                 input.masked {{ color: #777; background-color: #f2f2f2; }}\
                 label {{ display: block; margin-top: 10px; }}\
                 .toggle {{ display: flex; align-items: center; gap: 8px; font-size: 0.9rem; color: #444; margin-top: 5px; }}\
                 .toggle input {{ width: auto; margin: 0; }}\
                 button {{ width: 100%; padding: 12px; background-color: #2e7d32; color: white; border: none; cursor: pointer; }}\
                 button:hover {{ background-color: #1b5e20; }}\
             </style>\
             <script>\
                 function togglePasswordVisibility() {{\
                     var keepSavedPassword = document.getElementById('keep_saved_password');\
                     if (keepSavedPassword && keepSavedPassword.checked) return;\
                     var input = document.getElementById('password');\
                     if (!input) return;\
                     input.type = input.type === 'password' ? 'text' : 'password';\
                 }}\
                 function beginPasswordEdit() {{\
                     var keepSavedPassword = document.getElementById('keep_saved_password');\
                     if (!keepSavedPassword || !keepSavedPassword.checked) return;\
                     keepSavedPassword.checked = false;\
                     syncPasswordEditing();\
                 }}\
                 function syncPasswordEditing() {{\
                     var keepSavedPassword = document.getElementById('keep_saved_password');\
                     var passwordInput = document.getElementById('password');\
                     var showPassword = document.getElementById('show_password');\
                     if (!keepSavedPassword || !passwordInput) return;\
                     var isKeepingSavedPassword = keepSavedPassword.checked;\
                     passwordInput.readOnly = isKeepingSavedPassword;\
                     passwordInput.required = !isKeepingSavedPassword;\
                     if (isKeepingSavedPassword) {{\
                         passwordInput.value = '*******';\
                         passwordInput.type = 'password';\
                         passwordInput.classList.add('masked');\
                         if (showPassword) {{\
                             showPassword.checked = false;\
                             showPassword.disabled = true;\
                         }}\
                     }} else {{\
                         if (passwordInput.value === '*******') {{\
                             passwordInput.value = '';\
                         }}\
                         passwordInput.classList.remove('masked');\
                         if (showPassword) {{\
                             showPassword.disabled = false;\
                         }}\
                     }}\
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
",
        ssid
    )
    .expect("page HTML exceeds capacity");

    if has_saved_password {
        page.push_str(
            "<label class=\"toggle\"><input type=\"checkbox\" id=\"keep_saved_password\" name=\"keep_saved_password\" value=\"1\" checked onclick=\"syncPasswordEditing()\">Use saved password</label>\
             <input type=\"password\" id=\"password\" name=\"password\" onfocus=\"beginPasswordEdit()\" onkeydown=\"beginPasswordEdit()\" onclick=\"beginPasswordEdit()\" onpaste=\"beginPasswordEdit()\">\
             <label class=\"toggle\"><input type=\"checkbox\" id=\"show_password\" onclick=\"togglePasswordVisibility()\">Show password</label>\
             <script>syncPasswordEditing();</script>\
",
        )
        .expect("page HTML exceeds capacity");
    } else {
        page.push_str(
            "<input type=\"password\" id=\"password\" name=\"password\" required>\
             <label class=\"toggle\"><input type=\"checkbox\" id=\"show_password\" onclick=\"togglePasswordVisibility()\">Show password</label>\
",
        )
        .expect("page HTML exceeds capacity");
    }

    for field in fields {
        field
            .render(&mut page)
            .unwrap_or_else(|_| panic!("custom field HTML exceeds page capacity"));
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
