//! A device abstraction for extra setup fields used by Wi-Fi auto-provisioning.
//! See the [`crate::wifi_auto::WifiCredentials`] docs for the core Wi-Fi types.

#[doc(hidden)]
#[macro_export]
macro_rules! __impl_wifi_auto_fields {
    (
        flash_block = $flash_block:path,
        error = $error:path,
        wifi_auto_field = $wifi_auto_field:path,
        form_data = $form_data:ty,
        html_buffer = $html_buffer:ty
        $(, flash_cfg = $flash_cfg:meta)?
    ) => {
        use core::{cell::RefCell, fmt::Write};
        use $crate::flash_block::FlashBlock as _;

        use heapless::String;
        use static_cell::StaticCell;

        fn wifi_auto_format_error() -> $error
        where
            $error: From<$crate::Error>,
        {
            <$error as From<$crate::Error>>::from($crate::Error::WifiAutoFormat)
        }

        enum TimezoneFieldStorage {
            $(#[cfg($flash_cfg)])?
            Flash(RefCell<$flash_block>),
            Memory(RefCell<Option<i32>>),
        }

        /// A timezone selection field for WiFi provisioning.
        ///
        /// Allows users to select their timezone from a dropdown during captive-portal setup.
        /// The selected offset (in minutes from UTC) is persisted in memory or flash and can be
        /// retrieved later by application code.
        ///
        /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
        pub struct TimezoneField {
            storage: TimezoneFieldStorage,
        }

        // SAFETY: WifiAuto fields are used from a single-threaded Embassy executor. These
        // field instances are never accessed from interrupts, and access is cooperative.
        // Sync is required so the field can be stored behind static WifiAutoField references.
        unsafe impl Sync for TimezoneField {}

        /// Static for [`TimezoneField`].
        ///
        /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
        pub struct TimezoneFieldStatic {
            timezone_field_cell: StaticCell<TimezoneField>,
        }

        impl TimezoneFieldStatic {
            const fn new() -> Self {
                Self {
                    timezone_field_cell: StaticCell::new(),
                }
            }
        }

        impl TimezoneField {
            /// Create static resources for [`TimezoneField`].
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            #[must_use]
            pub const fn new_static() -> TimezoneFieldStatic {
                TimezoneFieldStatic::new()
            }

            /// Initialize a timezone field backed by in-memory state.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn new_in_memory(
                timezone_field_static: &'static TimezoneFieldStatic,
            ) -> &'static Self {
                timezone_field_static.timezone_field_cell.init(Self {
                    storage: TimezoneFieldStorage::Memory(RefCell::new(None)),
                })
            }

            /// Initialize a timezone field backed by a flash block.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            $(#[cfg($flash_cfg)])?
            pub fn new_with_flash(
                timezone_field_static: &'static TimezoneFieldStatic,
                timezone_flash_block: $flash_block,
            ) -> &'static Self {
                timezone_field_static.timezone_field_cell.init(Self {
                    storage: TimezoneFieldStorage::Flash(RefCell::new(timezone_flash_block)),
                })
            }

            /// Load the stored timezone offset in minutes from UTC.
            ///
            /// Returns `None` if no timezone has been configured yet.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn offset_minutes(&self) -> core::result::Result<Option<i32>, $error> {
                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TimezoneFieldStorage::Flash(timezone_flash_block) => {
                        timezone_flash_block.borrow_mut().load::<i32>()
                    }
                    TimezoneFieldStorage::Memory(offset_minutes) => Ok(*offset_minutes.borrow()),
                }
            }

            /// Save a timezone offset in minutes from UTC.
            ///
            /// This allows programmatic updates to timezone settings.
            ///
            /// For flash-backed fields, this only writes when the value changes to reduce
            /// unnecessary flash wear.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn set_offset_minutes(
                &self,
                offset_minutes: i32,
            ) -> core::result::Result<(), $error> {
                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TimezoneFieldStorage::Flash(timezone_flash_block) => {
                        let current_offset_minutes = timezone_flash_block.borrow_mut().load::<i32>()?;
                        if current_offset_minutes != Some(offset_minutes) {
                            timezone_flash_block.borrow_mut().save(&offset_minutes)?;
                        }
                        Ok(())
                    }
                    TimezoneFieldStorage::Memory(stored_offset_minutes) => {
                        *stored_offset_minutes.borrow_mut() = Some(offset_minutes);
                        Ok(())
                    }
                }
            }

            /// Clear the stored timezone offset.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn clear(&self) -> core::result::Result<(), $error> {
                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TimezoneFieldStorage::Flash(timezone_flash_block) => {
                        timezone_flash_block.borrow_mut().clear()
                    }
                    TimezoneFieldStorage::Memory(offset_minutes) => {
                        *offset_minutes.borrow_mut() = None;
                        Ok(())
                    }
                }
            }
        }

        impl $wifi_auto_field for TimezoneField {
            type Error = $error;

            fn render(&self, page: &mut $html_buffer) -> core::result::Result<(), $error> {
                let current_offset_minutes = self.offset_minutes()?.unwrap_or(0);
                let mut selected_option_rendered = false;
                write!(page, "<label for=\"timezone\">Time zone:</label>")
                    .map_err(|_| wifi_auto_format_error())?;
                write!(page, "<select id=\"timezone\" name=\"timezone\" required>")
                    .map_err(|_| wifi_auto_format_error())?;
                for timezone_option in TIMEZONE_OPTIONS {
                    let selected =
                        if !selected_option_rendered && timezone_option.minutes == current_offset_minutes {
                            selected_option_rendered = true;
                            " selected"
                        } else {
                            ""
                        };
                    write!(
                        page,
                        "<option value=\"{}\"{}>{}</option>",
                        timezone_option.minutes, selected, timezone_option.label
                    )
                    .map_err(|_| wifi_auto_format_error())?;
                }
                write!(page, "</select>").map_err(|_| wifi_auto_format_error())?;
                Ok(())
            }

            fn parse(&self, form: &$form_data) -> core::result::Result<(), $error> {
                let offset_minutes = form
                    .get("timezone")
                    .ok_or_else(wifi_auto_format_error)?
                    .parse::<i32>()
                    .map_err(|_| wifi_auto_format_error())?;
                assert!(
                    (-720..=840).contains(&offset_minutes),
                    "timezone offset_minutes must be within -720..=840"
                );
                self.set_offset_minutes(offset_minutes)
            }

            fn is_satisfied(&self) -> core::result::Result<bool, $error> {
                Ok(self.offset_minutes()?.is_some())
            }
        }

        struct TimezoneOption {
            minutes: i32,
            label: &'static str,
        }

        const TIMEZONE_OPTIONS: &[TimezoneOption] = &[
            TimezoneOption {
                minutes: -720,
                label: "Baker Island (UTC-12:00)",
            },
            TimezoneOption {
                minutes: -660,
                label: "American Samoa (UTC-11:00)",
            },
            TimezoneOption {
                minutes: -600,
                label: "Honolulu (UTC-10:00)",
            },
            TimezoneOption {
                minutes: -540,
                label: "Anchorage, Alaska ST (UTC-09:00)",
            },
            TimezoneOption {
                minutes: -480,
                label: "Anchorage, Alaska DT (UTC-08:00)",
            },
            TimezoneOption {
                minutes: -480,
                label: "Los Angeles, San Francisco, Seattle ST (UTC-08:00)",
            },
            TimezoneOption {
                minutes: -420,
                label: "Los Angeles, San Francisco, Seattle DT (UTC-07:00)",
            },
            TimezoneOption {
                minutes: -420,
                label: "Denver, Phoenix ST (UTC-07:00)",
            },
            TimezoneOption {
                minutes: -360,
                label: "Denver DT (UTC-06:00)",
            },
            TimezoneOption {
                minutes: -360,
                label: "Chicago, Dallas, Mexico City ST (UTC-06:00)",
            },
            TimezoneOption {
                minutes: -300,
                label: "Chicago, Dallas DT (UTC-05:00)",
            },
            TimezoneOption {
                minutes: -300,
                label: "New York, Toronto, Bogota ST (UTC-05:00)",
            },
            TimezoneOption {
                minutes: -240,
                label: "New York, Toronto DT (UTC-04:00)",
            },
            TimezoneOption {
                minutes: -240,
                label: "Santiago, Halifax ST (UTC-04:00)",
            },
            TimezoneOption {
                minutes: -210,
                label: "St. John's, Newfoundland ST (UTC-03:30)",
            },
            TimezoneOption {
                minutes: -180,
                label: "Buenos Aires, Sao Paulo (UTC-03:00)",
            },
            TimezoneOption {
                minutes: -120,
                label: "South Georgia (UTC-02:00)",
            },
            TimezoneOption {
                minutes: -60,
                label: "Azores ST (UTC-01:00)",
            },
            TimezoneOption {
                minutes: 0,
                label: "London, Lisbon ST (UTC+00:00)",
            },
            TimezoneOption {
                minutes: 60,
                label: "London, Paris, Berlin DT (UTC+01:00)",
            },
            TimezoneOption {
                minutes: 60,
                label: "Paris, Berlin, Rome ST (UTC+01:00)",
            },
            TimezoneOption {
                minutes: 120,
                label: "Paris, Berlin, Rome DT (UTC+02:00)",
            },
            TimezoneOption {
                minutes: 120,
                label: "Athens, Cairo, Johannesburg ST (UTC+02:00)",
            },
            TimezoneOption {
                minutes: 180,
                label: "Athens DT (UTC+03:00)",
            },
            TimezoneOption {
                minutes: 180,
                label: "Moscow, Istanbul, Nairobi (UTC+03:00)",
            },
            TimezoneOption {
                minutes: 240,
                label: "Dubai, Baku (UTC+04:00)",
            },
            TimezoneOption {
                minutes: 270,
                label: "Tehran ST (UTC+04:30)",
            },
            TimezoneOption {
                minutes: 300,
                label: "Karachi, Tashkent (UTC+05:00)",
            },
            TimezoneOption {
                minutes: 330,
                label: "Mumbai, Delhi (UTC+05:30)",
            },
            TimezoneOption {
                minutes: 345,
                label: "Kathmandu (UTC+05:45)",
            },
            TimezoneOption {
                minutes: 360,
                label: "Dhaka, Almaty (UTC+06:00)",
            },
            TimezoneOption {
                minutes: 390,
                label: "Yangon (UTC+06:30)",
            },
            TimezoneOption {
                minutes: 420,
                label: "Bangkok, Jakarta (UTC+07:00)",
            },
            TimezoneOption {
                minutes: 480,
                label: "Singapore, Hong Kong, Beijing (UTC+08:00)",
            },
            TimezoneOption {
                minutes: 525,
                label: "Eucla, Australia (UTC+08:45)",
            },
            TimezoneOption {
                minutes: 540,
                label: "Tokyo, Seoul (UTC+09:00)",
            },
            TimezoneOption {
                minutes: 570,
                label: "Adelaide ST (UTC+09:30)",
            },
            TimezoneOption {
                minutes: 600,
                label: "Sydney, Melbourne ST (UTC+10:00)",
            },
            TimezoneOption {
                minutes: 630,
                label: "Adelaide DT (UTC+10:30)",
            },
            TimezoneOption {
                minutes: 660,
                label: "Sydney, Melbourne DT (UTC+11:00)",
            },
            TimezoneOption {
                minutes: 720,
                label: "Auckland, Fiji ST (UTC+12:00)",
            },
            TimezoneOption {
                minutes: 780,
                label: "Auckland DT (UTC+13:00)",
            },
            TimezoneOption {
                minutes: 840,
                label: "Kiribati (UTC+14:00)",
            },
        ];

        enum TextFieldStorage<const N: usize> {
            $(#[cfg($flash_cfg)])?
            Flash(RefCell<$flash_block>),
            Memory(RefCell<Option<String<N>>>),
        }

        /// A generic text input field for collecting user input during WiFi provisioning.
        ///
        /// Presents a customizable text input box in the captive portal that validates and
        /// stores user-provided text in memory or flash.
        ///
        /// Multiple `TextField` instances can be created with different labels and field names
        /// to collect additional configuration beyond WiFi credentials.
        ///
        /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
        pub struct TextField<const N: usize> {
            storage: TextFieldStorage<N>,
            field_name: &'static str,
            label: &'static str,
            default_value: &'static str,
        }

        // SAFETY: WifiAuto fields are used from a single-threaded Embassy executor. These
        // field instances are never accessed from interrupts, and access is cooperative.
        // Sync is required so the field can be stored behind static WifiAutoField references.
        unsafe impl<const N: usize> Sync for TextField<N> {}

        /// Static for [`TextField`].
        ///
        /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
        pub struct TextFieldStatic<const N: usize> {
            text_field_cell: StaticCell<TextField<N>>,
        }

        impl<const N: usize> TextFieldStatic<N> {
            const fn new() -> Self {
                Self {
                    text_field_cell: StaticCell::new(),
                }
            }
        }

        impl<const N: usize> TextField<N> {
            /// Create static resources for [`TextField`].
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            #[must_use]
            pub const fn new_static() -> TextFieldStatic<N> {
                TextFieldStatic::new()
            }

            /// Initialize a text field backed by in-memory state.
            ///
            /// The maximum text length is determined by the generic parameter `N`.
            ///
            /// Parameters:
            ///
            /// - `text_field_static`: Static resources for initialization.
            /// - `field_name`: HTML form field name (for example, `device_name`).
            /// - `label`: HTML label text shown in the captive portal form.
            /// - `default_value`: Initial value if nothing has been configured yet.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn new_in_memory(
                text_field_static: &'static TextFieldStatic<N>,
                field_name: &'static str,
                label: &'static str,
                default_value: &'static str,
            ) -> &'static Self {
                text_field_static.text_field_cell.init(Self {
                    storage: TextFieldStorage::Memory(RefCell::new(None)),
                    field_name,
                    label,
                    default_value,
                })
            }

            /// Initialize a text field backed by a flash block.
            ///
            /// The maximum text length is determined by the generic parameter `N`.
            ///
            /// Parameters:
            ///
            /// - `text_field_static`: Static resources for initialization.
            /// - `text_flash_block`: Flash block for persistent storage.
            /// - `field_name`: HTML form field name (for example, `device_name`).
            /// - `label`: HTML label text shown in the captive portal form.
            /// - `default_value`: Initial value if nothing has been configured yet.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            $(#[cfg($flash_cfg)])?
            pub fn new_with_flash(
                text_field_static: &'static TextFieldStatic<N>,
                text_flash_block: $flash_block,
                field_name: &'static str,
                label: &'static str,
                default_value: &'static str,
            ) -> &'static Self {
                text_field_static.text_field_cell.init(Self {
                    storage: TextFieldStorage::Flash(RefCell::new(text_flash_block)),
                    field_name,
                    label,
                    default_value,
                })
            }

            /// Return the current text, if present.
            ///
            /// Returns `None` when no text has been configured yet.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn text(&self) -> core::result::Result<Option<String<N>>, $error> {
                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TextFieldStorage::Flash(text_flash_block) => {
                        text_flash_block.borrow_mut().load::<String<N>>()
                    }
                    TextFieldStorage::Memory(text) => Ok(text.borrow().clone()),
                }
            }

            /// Set the current text.
            ///
            /// This allows programmatic updates to the field value.
            ///
            /// The text must be non-empty and fit within the maximum length `N`.
            ///
            /// See the [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
            pub fn set_text(&self, text: &str) -> core::result::Result<(), $error> {
                assert!(
                    !text.is_empty() && text.len() <= N,
                    "text must be non-empty and fit field capacity"
                );
                let mut field_text = String::<N>::new();
                field_text
                    .push_str(text)
                    .expect("text must fit field capacity");

                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TextFieldStorage::Flash(text_flash_block) => {
                        text_flash_block.borrow_mut().save(&field_text)
                    }
                    TextFieldStorage::Memory(stored_text) => {
                        *stored_text.borrow_mut() = Some(field_text);
                        Ok(())
                    }
                }
            }

            /// Clear the current text.
            pub fn clear(&self) -> core::result::Result<(), $error> {
                match &self.storage {
                    $(#[cfg($flash_cfg)])?
                    TextFieldStorage::Flash(text_flash_block) => {
                        text_flash_block.borrow_mut().clear()
                    }
                    TextFieldStorage::Memory(text) => {
                        *text.borrow_mut() = None;
                        Ok(())
                    }
                }
            }
        }

        impl<const N: usize> $wifi_auto_field for TextField<N> {
            type Error = $error;

            fn render(&self, page: &mut $html_buffer) -> core::result::Result<(), $error> {
                let current_text = self
                    .text()?
                    .filter(|stored_text| !stored_text.is_empty())
                    .unwrap_or_else(|| {
                        let mut default_text = String::<N>::new();
                        default_text
                            .push_str(self.default_value)
                            .expect("default value must fit field capacity");
                        default_text
                    });
                let escaped = escape_html::<256>(current_text.as_str());
                write!(
                    page,
                    "<label for=\"{}\">{}:</label>\
                     <input type=\"text\" id=\"{}\" name=\"{}\" value=\"{}\" maxlength=\"{}\" required>",
                    self.field_name, self.label, self.field_name, self.field_name, escaped, N
                )
                .map_err(|_| wifi_auto_format_error())?;
                Ok(())
            }

            fn parse(&self, form: &$form_data) -> core::result::Result<(), $error> {
                let value = form.get(self.field_name).ok_or_else(wifi_auto_format_error)?;
                let trimmed_value = value.trim();
                assert!(
                    !trimmed_value.is_empty() && trimmed_value.len() <= N,
                    "text field value must be non-empty and fit field capacity"
                );
                self.set_text(trimmed_value)
            }

            fn is_satisfied(&self) -> core::result::Result<bool, $error> {
                Ok(self
                    .text()?
                    .map(|field_text| !field_text.is_empty())
                    .unwrap_or(false))
            }
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
    };
}
