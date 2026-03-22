use tauri::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, Window, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ICON_ID: &str = "main-tray";
const TRAY_SHOW_ID: &str = "tray-show";
const TRAY_EXIT_ID: &str = "tray-exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMenuAction {
    ShowMainWindow,
    ExitApp,
}

pub fn tray_menu_action(id: &str) -> Option<TrayMenuAction> {
    match id {
        TRAY_SHOW_ID => Some(TrayMenuAction::ShowMainWindow),
        TRAY_EXIT_ID => Some(TrayMenuAction::ExitApp),
        _ => None,
    }
}

pub fn should_hide_window_on_close(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_tray_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match tray_menu_action(event.id().as_ref()) {
        Some(TrayMenuAction::ShowMainWindow) => show_main_window(app),
        Some(TrayMenuAction::ExitApp) => app.exit(0),
        None => {}
    }
}

fn handle_tray_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
        | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => show_main_window(tray.app_handle()),
        _ => {}
    }
}

pub fn create_tray_icon<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let show_item = MenuItem::with_id(app, TRAY_SHOW_ID, "显示主窗口", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let exit_item = MenuItem::with_id(app, TRAY_EXIT_ID, "退出应用", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &separator, &exit_item])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&menu)
        .tooltip("Kiro Account Manager")
        .show_menu_on_left_click(false)
        .on_menu_event(handle_tray_menu_event)
        .on_tray_icon_event(handle_tray_icon_event);

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)
}

pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if should_hide_window_on_close(window.label()) {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{should_hide_window_on_close, tray_menu_action, TrayMenuAction};

    #[test]
    fn tray_behavior_maps_known_menu_ids() {
        assert_eq!(
            tray_menu_action("tray-show"),
            Some(TrayMenuAction::ShowMainWindow)
        );
        assert_eq!(tray_menu_action("tray-exit"), Some(TrayMenuAction::ExitApp));
        assert_eq!(tray_menu_action("unknown"), None);
    }

    #[test]
    fn tray_behavior_only_hides_main_window_close_requests() {
        assert!(should_hide_window_on_close("main"));
        assert!(!should_hide_window_on_close("auth-popup"));
    }
}
