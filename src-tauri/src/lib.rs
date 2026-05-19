mod modules;

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::Context;
use modules::{
    dictionary::{infer_entry_kind, is_single_word, lemmatize_word, normalize_text, Dictionary},
    models::{
        AppConfig, LookupErrorEvent, LookupEvent, LookupLoadingEvent, LookupPayload, LookupResult,
        ManualLookupRequest, PhraseLookupPayload, StarredQuery, ToggleStarredResponse,
        WordLookupPayload,
    },
    selection::capture_selected_text,
    store::Store,
    translator::{
        extract_keywords, local_translate_text, online_lookup_word, online_translate_text,
    },
};
use once_cell::sync::{Lazy, OnceCell};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
        WM_LBUTTONUP, WM_MBUTTONUP, WM_XBUTTONUP,
    },
};

static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();
static MOUSE_TRIGGER_DEBOUNCE: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
static OUTSIDE_CLICK_DEBOUNCE: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));

struct AppState {
    dictionary: Dictionary,
    store: Store,
    popup_drag_until: Mutex<Option<Instant>>,
}

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    state.store.config().map_err(stringify_error)
}

#[tauri::command]
fn save_config(state: tauri::State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
    state.store.save_config(&config).map_err(stringify_error)
}

#[tauri::command]
fn trigger_lookup(app: tauri::AppHandle) -> Result<(), String> {
    launch_lookup(&app).map_err(stringify_error)
}

#[tauri::command]
fn manual_lookup(
    state: tauri::State<'_, AppState>,
    request: ManualLookupRequest,
) -> Result<LookupResult, String> {
    build_lookup_result(&state, &request.text).map_err(stringify_error)
}

#[tauri::command]
fn list_starred(
    state: tauri::State<'_, AppState>,
    query: Option<StarredQuery>,
) -> Result<Vec<modules::models::StarredEntry>, String> {
    state
        .store
        .list_starred(query.unwrap_or_default())
        .map_err(stringify_error)
}

#[tauri::command]
fn toggle_starred(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: LookupPayload,
) -> Result<ToggleStarredResponse, String> {
    let response = state
        .store
        .toggle_starred(&payload)
        .map_err(stringify_error)?;
    emit_starred_changed(&app).map_err(stringify_error)?;
    Ok(response)
}

#[tauri::command]
fn remove_starred(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.store.remove_starred(&id).map_err(stringify_error)?;
    emit_starred_changed(&app).map_err(stringify_error)
}

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    open_main_window(&app).map_err(stringify_error)
}

#[tauri::command]
fn start_popup_drag(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut drag_until) = state.popup_drag_until.lock() {
        *drag_until = Some(Instant::now() + Duration::from_millis(1200));
    }

    let popup = app
        .get_webview_window("popup")
        .ok_or_else(|| "找不到释义弹窗".to_string())?;
    popup.start_dragging().map_err(stringify_error)
}

#[tauri::command]
fn resize_popup_to_content(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    content_height: f64,
) -> Result<(), String> {
    let config = state.store.config().map_err(stringify_error)?;
    let popup = app
        .get_webview_window("popup")
        .ok_or_else(|| "找不到释义弹窗".to_string())?;
    resize_popup_window_to_content(&popup, &config, content_height).map_err(stringify_error)
}

fn launch_lookup(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let config = app.state::<AppState>().store.config()?;
    let captured = capture_selected_text(config.max_text_length)?;

    show_popup_loading_window(app, captured.anchor.x, captured.anchor.y, &captured.text)?;

    let app_handle = app.clone();
    let text = captured.text.clone();
    let anchor = captured.anchor.clone();

    std::thread::spawn(move || {
        let outcome = {
            let state = app_handle.state::<AppState>();
            build_lookup_result(&state, &text)
        };

        match outcome {
            Ok(result) => {
                let _ = emit_lookup_result(&app_handle, result);
            }
            Err(error) => {
                let _ = emit_lookup_error(&app_handle, Some(anchor), Some(text), error.to_string());
            }
        }
    });

    Ok(())
}

fn spawn_lookup_from_global_trigger(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        if let Err(error) = launch_lookup(&app) {
            let _ = emit_lookup_error(&app, None, None, error.to_string());
        }
    });
}

fn build_lookup_result(state: &AppState, text: &str) -> anyhow::Result<LookupResult> {
    let normalized = normalize_text(text);
    if normalized.is_empty() {
        anyhow::bail!("没能抓到选中文本，请确认文本仍保持选中状态");
    }

    let payload = if is_single_word(&normalized) {
        build_word_payload(&state.dictionary, text)?
    } else {
        build_phrase_payload(&state.dictionary, text)
    };

    let is_starred = state.store.is_starred(payload.normalized_text())?;
    Ok(LookupResult {
        payload,
        is_starred,
    })
}

fn build_word_payload(dictionary: &Dictionary, text: &str) -> anyhow::Result<LookupPayload> {
    let normalized = normalize_text(text);
    let lemma = lemmatize_word(dictionary, &normalized);

    if let Some(entry) = dictionary.get(&lemma) {
        return Ok(LookupPayload::Word(WordLookupPayload {
            source_text: text.trim().to_string(),
            normalized_text: lemma.clone(),
            lemma,
            phonetic: entry.phonetic.clone(),
            senses: entry.senses.clone(),
        }));
    }

    if let Ok(Some(entry)) = online_lookup_word(&lemma) {
        return Ok(LookupPayload::Word(WordLookupPayload {
            source_text: text.trim().to_string(),
            normalized_text: entry.lemma.clone(),
            lemma: entry.lemma,
            phonetic: entry.phonetic,
            senses: entry.senses,
        }));
    }

    Ok(LookupPayload::Word(WordLookupPayload {
        source_text: text.trim().to_string(),
        normalized_text: lemma.clone(),
        lemma,
        phonetic: None,
        senses: vec![modules::models::DictionarySense {
            part_of_speech: "未知".to_string(),
            definitions: vec!["未找到释义".to_string()],
        }],
    }))
}

fn build_phrase_payload(dictionary: &Dictionary, text: &str) -> LookupPayload {
    let normalized = normalize_text(text);
    let translation = online_translate_text(text)
        .ok()
        .flatten()
        .unwrap_or_else(|| local_translate_text(dictionary, text));
    let payload = PhraseLookupPayload {
        source_text: text.trim().to_string(),
        normalized_text: normalized,
        translation,
        keywords: extract_keywords(dictionary, text, 4),
    };

    match infer_entry_kind(text) {
        "sentence" => LookupPayload::Sentence(payload),
        _ => LookupPayload::Phrase(payload),
    }
}

fn with_popup_window<F>(app: &tauri::AppHandle, handler: F) -> anyhow::Result<()>
where
    F: FnOnce(&tauri::WebviewWindow) -> anyhow::Result<()>,
{
    let popup = app.get_webview_window("popup").context("找不到释义弹窗")?;
    handler(&popup)
}

fn position_and_show_popup(
    popup: &tauri::WebviewWindow,
    cursor_x: i32,
    cursor_y: i32,
) -> anyhow::Result<()> {
    let x = cursor_x + 18;
    let y = cursor_y + 18;
    popup.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;
    popup.show()?;
    popup.unminimize()?;
    Ok(())
}

fn show_popup_loading_window(
    app: &tauri::AppHandle,
    cursor_x: i32,
    cursor_y: i32,
    text: &str,
) -> anyhow::Result<()> {
    let config = app.state::<AppState>().store.config()?;
    with_popup_window(app, |popup| {
        resize_popup_window(popup, &config)?;
        popup.emit(
            "lookup-loading",
            LookupLoadingEvent {
                anchor: modules::models::AnchorPoint {
                    x: cursor_x,
                    y: cursor_y,
                },
                text: text.to_string(),
            },
        )?;
        position_and_show_popup(popup, cursor_x, cursor_y)
    })
}

fn resize_popup_window(popup: &tauri::WebviewWindow, config: &AppConfig) -> anyhow::Result<()> {
    popup.set_size(Size::Logical(LogicalSize {
        width: config.popup_width.clamp(300, 520) as f64,
        height: config.popup_height.clamp(120, 680) as f64,
    }))?;
    Ok(())
}

fn resize_popup_window_to_content(
    popup: &tauri::WebviewWindow,
    config: &AppConfig,
    content_height: f64,
) -> anyhow::Result<()> {
    let max_height = config.popup_height.clamp(120, 680) as f64;
    let target_height = content_height.ceil().clamp(120.0, max_height);
    popup.set_size(Size::Logical(LogicalSize {
        width: config.popup_width.clamp(300, 520) as f64,
        height: target_height,
    }))?;
    Ok(())
}

fn emit_lookup_result(app: &tauri::AppHandle, result: LookupResult) -> anyhow::Result<()> {
    with_popup_window(app, |popup| {
        popup.emit(
            "lookup-result",
            LookupEvent {
                anchor: modules::models::AnchorPoint { x: 0, y: 0 },
                result,
            },
        )?;
        Ok(())
    })
}

fn emit_lookup_error(
    app: &tauri::AppHandle,
    anchor: Option<modules::models::AnchorPoint>,
    text: Option<String>,
    message: String,
) -> anyhow::Result<()> {
    with_popup_window(app, |popup| {
        popup.emit(
            "lookup-error",
            LookupErrorEvent {
                anchor,
                text,
                message,
            },
        )?;
        popup.show()?;
        Ok(())
    })
}

fn emit_starred_changed(app: &tauri::AppHandle) -> anyhow::Result<()> {
    app.emit("starred-changed", ())?;
    Ok(())
}

fn open_main_window(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let window = app.get_webview_window("main").context("找不到主窗口")?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

fn build_popup_window(app: &tauri::AppHandle) -> anyhow::Result<()> {
    if app.get_webview_window("popup").is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("index.html".into()))
        .title("Lookup Popup")
        .inner_size(380.0, 400.0)
        .visible(false)
        .decorations(false)
        .shadow(true)
        .transparent(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .build()?;

    Ok(())
}

fn build_tray(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let open = MenuItem::with_id(app, "open-main", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Reading Assistant Pro")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open-main" => {
                let _ = open_main_window(app);
            }
            "quit" => {
                app.cleanup_before_exit();
                std::process::exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn create_state(app: &tauri::AppHandle) -> anyhow::Result<AppState> {
    let data_dir = app.path().app_data_dir().context("无法解析应用数据目录")?;
    let store = Store::new(&data_dir)?;
    let dictionary = Dictionary::load()?;
    Ok(AppState {
        dictionary,
        store,
        popup_drag_until: Mutex::new(None),
    })
}

fn popup_drag_in_progress(state: &AppState) -> bool {
    state
        .popup_drag_until
        .lock()
        .ok()
        .and_then(|drag_until| *drag_until)
        .is_some_and(|drag_until| Instant::now() <= drag_until)
}

fn stringify_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn global_trigger_matches(config: &AppConfig, trigger_source: &str) -> bool {
    config.trigger_source == trigger_source
}

fn allow_mouse_trigger() -> bool {
    let Ok(mut debounce_until) = MOUSE_TRIGGER_DEBOUNCE.lock() else {
        return false;
    };

    let now = Instant::now();
    if debounce_until.is_some_and(|deadline| deadline > now) {
        return false;
    }

    *debounce_until = Some(now + Duration::from_millis(350));
    true
}

fn dispatch_mouse_trigger(trigger_source: &str) {
    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };

    let state = app.state::<AppState>();
    let Ok(config) = state.store.config() else {
        return;
    };

    if !global_trigger_matches(&config, trigger_source) || !allow_mouse_trigger() {
        return;
    }

    spawn_lookup_from_global_trigger(app);
}

fn hide_popup_after_outside_click() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };

    let state = app.state::<AppState>();
    if popup_drag_in_progress(&state) {
        return;
    }

    let Ok(config) = state.store.config() else {
        return;
    };
    if !config.close_on_focus_loss {
        return;
    }

    let Some(popup) = app.get_webview_window("popup") else {
        return;
    };
    if !popup.is_visible().unwrap_or(false) {
        return;
    }

    let Ok(position) = popup.outer_position() else {
        return;
    };
    let Ok(size) = popup.outer_size() else {
        return;
    };

    let mut cursor = POINT::default();
    let cursor_ok = unsafe { GetCursorPos(&mut cursor).is_ok() };
    if !cursor_ok {
        return;
    }

    let x = cursor.x;
    let y = cursor.y;
    let left = position.x;
    let top = position.y;
    let right = left + size.width as i32;
    let bottom = top + size.height as i32;
    let inside_popup = x >= left && x <= right && y >= top && y <= bottom;

    if !inside_popup {
        let _ = popup.hide();
    }
}

fn allow_outside_click_check() -> bool {
    let Ok(mut debounce_until) = OUTSIDE_CLICK_DEBOUNCE.lock() else {
        return false;
    };

    let now = Instant::now();
    if debounce_until.is_some_and(|deadline| deadline > now) {
        return false;
    }

    *debounce_until = Some(now + Duration::from_millis(80));
    true
}

fn dispatch_mouse_trigger_async(trigger_source: &'static str) {
    std::thread::spawn(move || dispatch_mouse_trigger(trigger_source));
}

fn hide_popup_after_outside_click_async() {
    if !allow_outside_click_check() {
        return;
    }

    std::thread::spawn(hide_popup_after_outside_click);
}

unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        match w_param.0 as u32 {
            WM_LBUTTONUP => {
                hide_popup_after_outside_click_async();
            }
            WM_MBUTTONUP => {
                dispatch_mouse_trigger_async("mouse_middle");
            }
            WM_XBUTTONUP => {
                let info = &*(l_param.0 as *const MSLLHOOKSTRUCT);
                let x_button = hiword(info.mouseData);
                if x_button == 1 {
                    dispatch_mouse_trigger_async("mouse_x1");
                } else if x_button == 2 {
                    dispatch_mouse_trigger_async("mouse_x2");
                }
            }
            _ => {}
        }
    }

    unsafe { CallNextHookEx(None, code, w_param, l_param) }
}

fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xffff) as u16
}

fn install_mouse_hook() {
    std::thread::spawn(|| unsafe {
        let module: HINSTANCE = GetModuleHandleW(None).unwrap_or_default().into();
        let hook: HHOOK =
            match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), Some(module), 0) {
                Ok(hook) => hook,
                Err(_) => return,
            };

        if hook.is_invalid() {
            return;
        }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = UnhookWindowsHookEx(hook);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let state = create_state(&app.handle())?;
            app.manage(state);
            build_popup_window(&app.handle())?;
            build_tray(&app.handle())?;

            let _ = APP_HANDLE.set(app.handle().clone());
            install_mouse_hook();

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let window = window.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = window.hide();
                    });
                }
            }

            if window.label() == "popup" {
                match event {
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    WindowEvent::Focused(false) => {
                        let state = window.app_handle().state::<AppState>();
                        if popup_drag_in_progress(&state) {
                            return;
                        }

                        let should_close = state
                            .store
                            .config()
                            .map(|config| config.close_on_focus_loss)
                            .unwrap_or(true);
                        if should_close {
                            let _ = window.hide();
                        }
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            trigger_lookup,
            manual_lookup,
            list_starred,
            toggle_starred,
            remove_starred,
            show_main_window,
            start_popup_drag,
            resize_popup_to_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
