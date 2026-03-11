use core::cell::RefCell;

use defmt::{info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write as _;
use heapless::Vec;
use static_cell::StaticCell;

use crate::Result;
use device_envoy_core::wifi_auto::{HtmlBuffer, WifiCredentials};

/// Traits for custom extra information that [`WifiAutoRp`](crate::wifi_auto::WifiAutoRp) can ask the
/// user for on its setup web page. Supports HTML snippets.
///
/// Implement this trait to collect additional configuration beyond WiFi credentials
/// during the captive portal setup. Fields must be `Sync` since they're shared across
/// async tasks.
///
/// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
///
/// This trait forwards to [`device_envoy_core::wifi_auto::WifiAutoField`] with
/// `Error = crate::Error` and adds `Sync` for static shared usage in RP tasks.
pub trait WifiAutoField:
    device_envoy_core::wifi_auto::WifiAutoField<Error = crate::Error> + Sync
{
}

impl<T> WifiAutoField for T where
    T: device_envoy_core::wifi_auto::WifiAutoField<Error = crate::Error> + Sync
{
}

static CREDENTIAL_CHANNEL: Channel<CriticalSectionRawMutex, WifiCredentials, 1> = Channel::new();

#[derive(Clone)]
struct FormState {
    defaults: Option<WifiCredentials>,
}

static FORM_STATE: Mutex<CriticalSectionRawMutex, RefCell<FormState>> =
    Mutex::new(RefCell::new(FormState { defaults: None }));

static FORM_FIELDS: Mutex<CriticalSectionRawMutex, RefCell<&'static [&'static dyn WifiAutoField]>> =
    Mutex::new(RefCell::new(&[]));

pub async fn collect_credentials(
    stack: &'static Stack<'static>,
    spawner: Spawner,
    defaults: Option<&WifiCredentials>,
    fields: &'static [&'static dyn WifiAutoField],
) -> Result<WifiCredentials> {
    info!(
        "WifiAutoRp portal registering {} custom fields",
        fields.len()
    );
    FORM_STATE.lock(|state| {
        state.borrow_mut().defaults = defaults.cloned();
    });
    FORM_FIELDS.lock(|slot| {
        *slot.borrow_mut() = fields;
    });

    unwrap!(spawner.spawn(http_server_task(stack)));

    let submission = CREDENTIAL_CHANNEL.receive().await;
    Ok(submission)
}

#[embassy_executor::task]
async fn http_server_task(stack: &'static Stack<'static>) -> ! {
    info!("WifiAutoRp HTTP portal starting");

    static RX_BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();
    static TX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
    static REQUEST_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();

    let rx_buffer = RX_BUFFER.init([0; 2048]);
    let tx_buffer = TX_BUFFER.init([0; 4096]);
    let request = REQUEST_BUFFER.init([0; 1024]);

    loop {
        let mut socket = TcpSocket::new(*stack, rx_buffer, tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(30)));

        info!("Waiting for HTTP connection...");
        if let Err(err) = socket.accept(80).await {
            warn!("Accept error: {:?}", err);
            Timer::after_millis(500).await;
            continue;
        }

        let request_len = match socket.read(request).await {
            Ok(0) => {
                info!("Client closed connection");
                socket.flush().await.ok();
                socket.close();
                continue;
            }
            Ok(n) => n,
            Err(err) => {
                warn!("HTTP read error: {:?}", err);
                socket.flush().await.ok();
                socket.close();
                continue;
            }
        };

        let request_text = core::str::from_utf8(&request[..request_len]).unwrap_or("");
        let mut lines = request_text.lines();
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");

        let response = match method {
            "GET" => {
                let state_snapshot = FORM_STATE.lock(|state| state.borrow().clone());
                let fields_snapshot = FORM_FIELDS.lock(|fields| *fields.borrow());
                generate_config_page(state_snapshot.defaults.as_ref(), fields_snapshot)
                    .unwrap_or_else(|| static_page(generate_error_page()))
            }
            "POST" => {
                let state_snapshot = FORM_STATE.lock(|state| state.borrow().clone());
                let fields_snapshot = FORM_FIELDS.lock(|fields| *fields.borrow());
                if let Some(credentials) = parse_post(
                    request_text,
                    state_snapshot.defaults.as_ref(),
                    fields_snapshot,
                ) {
                    CREDENTIAL_CHANNEL.send(credentials).await;
                    static_page(generate_success_page())
                } else {
                    warn!("WifiAutoRp portal failed to parse POST");
                    static_page(generate_error_page())
                }
            }
            _ => static_page(generate_error_page()),
        };

        if let Err(err) = socket.write_all(response.as_bytes()).await {
            warn!("HTTP write error: {:?}", err);
        }

        socket.flush().await.ok();
        socket.close();
        Timer::after_millis(100).await;
    }
}

fn parse_post(
    request: &str,
    defaults: Option<&WifiCredentials>,
    fields: &[&'static dyn WifiAutoField],
) -> Option<WifiCredentials> {
    let core_fields = core_fields(fields)?;
    device_envoy_core::wifi_auto::parse_post(request, defaults, core_fields.as_slice())
}

fn generate_config_page(
    defaults: Option<&WifiCredentials>,
    fields: &[&'static dyn WifiAutoField],
) -> Option<HtmlBuffer> {
    info!("WifiAutoRp portal rendering {} fields", fields.len());
    let core_fields = core_fields(fields)?;
    Some(device_envoy_core::wifi_auto::generate_config_page(
        defaults,
        core_fields.as_slice(),
    ))
}

fn generate_success_page() -> &'static str {
    "HTTP/1.1 200 OK\r\n\
     Content-Type: text/html\r\n\
     Connection: close\r\n\
     \r\n\
     <!DOCTYPE html>\
     <html>\
     <head>\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Configuration Saved</title>\
         <style>\
             body { font-family: Arial, sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; text-align: center; }\
             h1 { color: #4CAF50; }\
         </style>\
     </head>\
     <body>\
         <h1>Configuration Saved!</h1>\
         <p>WiFi credentials have been received.</p>\
         <p>The device will restart and connect to your network.</p>\
         <p>You can close this page.</p>\
     </body>\
     </html>"
}

fn generate_error_page() -> &'static str {
    "HTTP/1.1 400 Bad Request\r\n\
     Content-Type: text/html\r\n\
     Connection: close\r\n\
     \r\n\
     <!DOCTYPE html>\
     <html>\
     <head>\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Error</title>\
         <style>\
             body { font-family: Arial, sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; text-align: center; }\
             h1 { color: #f44336; }\
         </style>\
     </head>\
     <body>\
         <h1>Error</h1>\
         <p>Failed to process your request.</p>\
         <p><a href=\"/\">Try again</a></p>\
     </body>\
     </html>"
}

fn static_page(content: &'static str) -> HtmlBuffer {
    let mut page = HtmlBuffer::new();
    page.push_str(content)
        .expect("static content exceeds page capacity");
    page
}

fn core_fields(
    fields: &[&'static dyn WifiAutoField],
) -> Option<Vec<&'static dyn device_envoy_core::wifi_auto::WifiAutoField<Error = crate::Error>, 16>>
{
    let mut core_fields: Vec<
        &'static dyn device_envoy_core::wifi_auto::WifiAutoField<Error = crate::Error>,
        16,
    > = Vec::new();
    for field in fields {
        if core_fields
            .push(*field as &'static dyn device_envoy_core::wifi_auto::WifiAutoField<Error = crate::Error>)
            .is_err()
        {
            warn!("WifiAutoRp portal has too many fields to parse");
            return None;
        }
    }
    Some(core_fields)
}
