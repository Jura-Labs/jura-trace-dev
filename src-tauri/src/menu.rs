// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native OS menu for Jura Trace.
//!
//! Builds a full menu bar that replaces Tauri v2's default auto-menu.  Every
//! predefined item users rely on (clipboard, window management, quit, about)
//! is re-added so clipboard shortcuts continue to work in the webview.
//!
//! # Event contract
//!
//! Custom items emit Tauri app-wide events on selection.  The frontend
//! (`+layout.svelte`) listens for these events and reacts accordingly.
//!
//! | Menu-item ID      | Tauri event emitted    | Payload              |
//! |-------------------|------------------------|----------------------|
//! | `go-protect`      | `menu:navigate`        | `"/protect"`         |
//! | `go-verify`       | `menu:navigate`        | `"/verify"`          |
//! | `go-monitor`      | `menu:navigate`        | `"/monitor"`         |
//! | `go-settings`     | `menu:navigate`        | `"/settings"`        |
//! | `go-help`         | `menu:navigate`        | `"/help"`            |
//! | `open-settings`   | `menu:navigate`        | `"/settings"`        |
//! | `check-updates`   | `menu:check-updates`   | *(none)*             |
//! | `send-feedback`   | `menu:show-feedback`   | *(none)*             |
//! | `search-help`     | `menu:search-help`     | *(none)*             |
//! | `zoom-in`         | `menu:zoom`            | `"in"`               |
//! | `zoom-out`        | `menu:zoom`            | `"out"`              |
//! | `zoom-reset`      | `menu:zoom`            | `"reset"`            |

use tauri::{
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    AppHandle, Emitter, Runtime,
};

/// Build and return the application menu.
///
/// Called from the `menu()` closure in `tauri::Builder`.  The resulting
/// `Menu` replaces Tauri v2's default auto-menu entirely.  All predefined
/// items that the webview renderer relies on (clipboard, undo/redo, window
/// controls, quit) are re-included so keyboard shortcuts remain functional.
///
/// # Event contract (additions)
///
/// | Menu-item ID        | Tauri event emitted | Payload               |
/// |---------------------|---------------------|-----------------------|
/// | `zoom-in`           | `menu:zoom`         | `"in"`                |
/// | `zoom-out`          | `menu:zoom`         | `"out"`               |
/// | `zoom-reset`        | `menu:zoom`         | `"reset"`             |
///
/// # Errors
/// Returns a `tauri::Error` if any menu item construction fails (e.g. a
/// platform does not support a particular predefined item).
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::menu::Menu<R>, tauri::Error> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    // ── About metadata ──────────────────────────────────────────────────────
    let about_meta = AboutMetadata {
        name: Some("Jura Trace".to_string()),
        version: Some(version),
        comments: Some("Know What's Real".to_string()),
        website: Some("https://juralabs.org".to_string()),
        website_label: Some("juralabs.org".to_string()),
        copyright: Some("Jura Labs CIC".to_string()),
        ..Default::default()
    };

    // ── Predefined items (shared across platform branches) ──────────────────

    // Edit submenu items
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;

    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .item(&undo)
        .item(&redo)
        .separator()
        .item(&cut)
        .item(&copy)
        .item(&paste)
        .item(&select_all)
        .build()?;

    // Go submenu (navigation shortcuts)
    let go_protect = MenuItemBuilder::with_id("go-protect", "Protect")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let go_verify = MenuItemBuilder::with_id("go-verify", "Verify")
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
    let go_monitor = MenuItemBuilder::with_id("go-monitor", "Monitor")
        .accelerator("CmdOrCtrl+3")
        .build(app)?;
    let go_settings = MenuItemBuilder::with_id("go-settings", "Settings")
        .accelerator("CmdOrCtrl+4")
        .build(app)?;
    let go_help = MenuItemBuilder::with_id("go-help", "Help").build(app)?;

    let go_submenu = SubmenuBuilder::new(app, "Go")
        .item(&go_protect)
        .item(&go_verify)
        .item(&go_monitor)
        .item(&go_settings)
        .separator()
        .item(&go_help)
        .build()?;

    // ── Platform-specific menu layout ────────────────────────────────────────
    #[cfg(target_os = "macos")]
    {
        // macOS: App submenu first (titled with the app name), then standard submenus.

        let about = PredefinedMenuItem::about(app, None, Some(about_meta))?;
        let check_updates =
            MenuItemBuilder::with_id("check-updates", "Check for Updates\u{2026}").build(app)?;
        let settings_item = MenuItemBuilder::with_id("open-settings", "Settings\u{2026}")
            .accelerator("CmdOrCtrl+,")
            .build(app)?;
        let services = PredefinedMenuItem::services(app, None)?;
        let hide = PredefinedMenuItem::hide(app, None)?;
        let hide_others = PredefinedMenuItem::hide_others(app, None)?;
        let show_all = PredefinedMenuItem::show_all(app, None)?;
        let quit = PredefinedMenuItem::quit(app, None)?;

        let app_submenu = SubmenuBuilder::new(app, "Jura Trace")
            .item(&about)
            .separator()
            .item(&check_updates)
            .separator()
            .item(&settings_item)
            .separator()
            .item(&services)
            .separator()
            .item(&hide)
            .item(&hide_others)
            .item(&show_all)
            .separator()
            .item(&quit)
            .build()?;

        // View submenu — includes text-size zoom items
        let fullscreen = PredefinedMenuItem::fullscreen(app, None)?;
        let zoom_in = MenuItemBuilder::with_id("zoom-in", "Increase Text Size")
            .accelerator("CmdOrCtrl+=")
            .build(app)?;
        let zoom_out = MenuItemBuilder::with_id("zoom-out", "Decrease Text Size")
            .accelerator("CmdOrCtrl+-")
            .build(app)?;
        let zoom_reset = MenuItemBuilder::with_id("zoom-reset", "Reset Text Size")
            .accelerator("CmdOrCtrl+0")
            .build(app)?;

        let view_submenu = SubmenuBuilder::new(app, "View")
            .item(&fullscreen)
            .separator()
            .item(&zoom_in)
            .item(&zoom_out)
            .item(&zoom_reset)
            .build()?;

        // Window submenu
        let minimise = PredefinedMenuItem::minimize(app, Some("Minimise"))?;
        let maximise = PredefinedMenuItem::maximize(app, Some("Maximise"))?;
        let close_window = PredefinedMenuItem::close_window(app, None)?;

        let window_submenu = SubmenuBuilder::new(app, "Window")
            .item(&minimise)
            .item(&maximise)
            .separator()
            .item(&close_window)
            .build()?;

        // Help submenu (macOS: no About here — it lives in the App submenu)
        let send_feedback =
            MenuItemBuilder::with_id("send-feedback", "Send Feedback\u{2026}").build(app)?;
        let search_help = MenuItemBuilder::with_id("search-help", "Search Help\u{2026}")
            .accelerator("CmdOrCtrl+Shift+F")
            .build(app)?;

        let help_submenu = SubmenuBuilder::new(app, "Help")
            .item(&search_help)
            .separator()
            .item(&send_feedback)
            .build()?;

        let menu = MenuBuilder::new(app)
            .item(&app_submenu)
            .item(&edit_submenu)
            .item(&go_submenu)
            .item(&view_submenu)
            .item(&window_submenu)
            .item(&help_submenu)
            .build()?;

        Ok(menu)
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Windows / Linux: File submenu for Settings + Exit; Help submenu for About.

        let settings_item = MenuItemBuilder::with_id("open-settings", "Settings\u{2026}")
            .accelerator("CmdOrCtrl+,")
            .build(app)?;
        let quit = PredefinedMenuItem::quit(app, Some("Exit"))?;
        let file_sep = PredefinedMenuItem::separator(app)?;

        let file_submenu = SubmenuBuilder::new(app, "File")
            .item(&settings_item)
            .item(&file_sep)
            .item(&quit)
            .build()?;

        // View submenu — fullscreen + text-size zoom items
        let fullscreen = PredefinedMenuItem::fullscreen(app, None)?;
        let zoom_in = MenuItemBuilder::with_id("zoom-in", "Increase Text Size")
            .accelerator("CmdOrCtrl+=")
            .build(app)?;
        let zoom_out = MenuItemBuilder::with_id("zoom-out", "Decrease Text Size")
            .accelerator("CmdOrCtrl+-")
            .build(app)?;
        let zoom_reset = MenuItemBuilder::with_id("zoom-reset", "Reset Text Size")
            .accelerator("CmdOrCtrl+0")
            .build(app)?;

        let view_submenu = SubmenuBuilder::new(app, "View")
            .item(&fullscreen)
            .separator()
            .item(&zoom_in)
            .item(&zoom_out)
            .item(&zoom_reset)
            .build()?;

        // Window submenu
        let minimise = PredefinedMenuItem::minimize(app, Some("Minimise"))?;
        let maximise = PredefinedMenuItem::maximize(app, Some("Maximise"))?;
        let close_window = PredefinedMenuItem::close_window(app, None)?;

        let window_submenu = SubmenuBuilder::new(app, "Window")
            .item(&minimise)
            .item(&maximise)
            .separator()
            .item(&close_window)
            .build()?;

        let about = PredefinedMenuItem::about(app, None, Some(about_meta))?;
        let check_updates =
            MenuItemBuilder::with_id("check-updates", "Check for Updates\u{2026}").build(app)?;
        let send_feedback =
            MenuItemBuilder::with_id("send-feedback", "Send Feedback\u{2026}").build(app)?;
        let search_help = MenuItemBuilder::with_id("search-help", "Search Help\u{2026}")
            .accelerator("CmdOrCtrl+Shift+F")
            .build(app)?;

        let help_submenu = SubmenuBuilder::new(app, "Help")
            .item(&search_help)
            .separator()
            .item(&check_updates)
            .item(&send_feedback)
            .separator()
            .item(&about)
            .build()?;

        let menu = MenuBuilder::new(app)
            .item(&file_submenu)
            .item(&edit_submenu)
            .item(&go_submenu)
            .item(&view_submenu)
            .item(&window_submenu)
            .item(&help_submenu)
            .build()?;

        Ok(menu)
    }
}

/// Handle a menu event by emitting a Tauri event to the webview.
///
/// Called from `tauri::Builder::on_menu_event`.  Maps each stable menu-item
/// ID to its corresponding frontend event.  Unknown IDs are silently ignored
/// so future additions or platform-predefined items do not cause panics.
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        // ── Navigation ───────────────────────────────────────────────────────
        "go-protect" | "open-protect" => {
            let _ = app.emit("menu:navigate", "/protect");
        }
        "go-verify" => {
            let _ = app.emit("menu:navigate", "/verify");
        }
        "go-monitor" => {
            let _ = app.emit("menu:navigate", "/monitor");
        }
        "go-settings" | "open-settings" => {
            let _ = app.emit("menu:navigate", "/settings");
        }
        "go-help" => {
            let _ = app.emit("menu:navigate", "/help");
        }
        // ── Actions ──────────────────────────────────────────────────────────
        "check-updates" => {
            let _ = app.emit("menu:check-updates", ());
        }
        "send-feedback" => {
            let _ = app.emit("menu:show-feedback", ());
        }
        "search-help" => {
            let _ = app.emit("menu:search-help", ());
        }
        // ── View — text-size zoom ─────────────────────────────────────────────
        "zoom-in" => {
            let _ = app.emit("menu:zoom", "in");
        }
        "zoom-out" => {
            let _ = app.emit("menu:zoom", "out");
        }
        "zoom-reset" => {
            let _ = app.emit("menu:zoom", "reset");
        }
        // All other IDs (predefined items handled natively by the OS) are ignored.
        _ => {}
    }
}
