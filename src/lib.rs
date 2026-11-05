//! # autumn-command-palette-plugin
//!
//! A ⌘K command palette for Autumn apps (autumn#3416). It is a widget pack
//! (autumn ADR 0030).
//!
//! - ⌘K or Ctrl+K, a [`Hotkey`] that you set, or the invoker button opens
//!   it.
//! - The server finds the commands. htmx sends the query and shows the
//!   results: groups, `kbd` shortcuts and recent items.
//! - The app fills a [`CommandRegistry`] with routes and actions. With the
//!   `admin` feature, `admin::admin_commands` adds the admin resources.
//! - With JavaScript off, the invoker is a link to a plain search page.
//!
//! The palette is a core `modal()` `<dialog>`. The input is an APG combobox.
//! The listbox uses the keyboard core (`data-autumn-activedescendant`). The
//! pack script has no arrow-key code.
//!
//! ```no_run
//! use autumn_command_palette_plugin::{Command, CommandPalette, CommandRegistry};
//!
//! let palette = CommandPalette::new(
//!     CommandRegistry::new()
//!         .add(Command::route("home", "Dashboard", "/"))
//!         .add(Command::action("sign-out", "Sign out", "/logout")),
//! );
//! // Install it: `autumn_web::app().plugin(palette.clone())`.
//! // Show it in the layout: `(palette)` in a maud template.
//! # let _ = palette;
//! ```

#[cfg(feature = "admin")]
pub mod admin;
mod command;
mod hotkey;
mod palette;
mod render;
pub mod route;
pub mod search;

pub use command::CommandRegistry;
pub use command::{
    ACTIONS_GROUP, Command, CommandId, CommandTarget, MAX_ID_LEN, ROUTES_GROUP, RegistryError,
    Visibility,
};
pub use hotkey::{Hotkey, HotkeyError};
pub use palette::{
    ASSETS, CommandPalette, Csrf, Labels, PACK_NAME, PALETTE_PATH, PLUGIN_NAME, PageLayout,
    demo_palette, pack,
};
pub use search::{Limits, ResultGroup, SearchResults};
