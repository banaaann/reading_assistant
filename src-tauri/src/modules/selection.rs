use std::{thread, time::Duration};

use anyhow::{anyhow, Context};
use clipboard_win::{formats, raw, Clipboard, Getter, Setter};
use windows::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            VK_CONTROL, VK_INSERT,
        },
        WindowsAndMessaging::{GetClassNameW, GetCursorPos, GetForegroundWindow},
    },
};

use crate::modules::models::AnchorPoint;

pub struct CapturedSelection {
    pub text: String,
    pub anchor: AnchorPoint,
}

#[derive(Default)]
struct ClipboardBackup {
    unicode_text: Option<String>,
}

pub fn capture_selected_text(max_len: usize) -> anyhow::Result<CapturedSelection> {
    ensure_safe_foreground_window()?;

    let backup = backup_clipboard().ok();
    let previous_text = backup.as_ref().and_then(|item| item.unicode_text.clone());
    let sequence_before = raw::seq_num();

    let capture_result = (|| -> anyhow::Result<String> {
        trigger_copy_shortcut()?;
        if let Ok(text) = wait_for_selection_text(sequence_before, previous_text.as_deref()) {
            return Ok(text);
        }

        trigger_copy_insert_shortcut()?;
        wait_for_selection_text(sequence_before, previous_text.as_deref())
    })();

    if let Some(snapshot) = backup.as_ref() {
        let _ = restore_clipboard(snapshot);
    }

    let captured_text = capture_result?;
    let text = captured_text.trim().to_string();
    if text.is_empty() {
        return Err(anyhow!("没能抓到选中文本，请确认文本仍保持选中状态"));
    }

    let truncated = text.chars().take(max_len).collect::<String>();
    let anchor = current_cursor_position()?;

    Ok(CapturedSelection {
        text: truncated,
        anchor,
    })
}

fn ensure_safe_foreground_window() -> anyhow::Result<()> {
    let class_name = foreground_window_class_name()?;
    let blocked = ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS"];
    if blocked.iter().any(|item| *item == class_name) {
        return Err(anyhow!(
            "当前焦点在终端窗口，已取消取词，避免中断正在运行的开发进程"
        ));
    }

    Ok(())
}

fn foreground_window_class_name() -> anyhow::Result<String> {
    let handle = unsafe { GetForegroundWindow() };
    if handle.0.is_null() {
        return Err(anyhow!("无法识别当前前台窗口"));
    }

    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(handle, &mut buffer) };
    if len == 0 {
        return Err(anyhow!("无法读取当前前台窗口类型"));
    }

    Ok(String::from_utf16_lossy(&buffer[..len as usize]))
}

fn backup_clipboard() -> anyhow::Result<ClipboardBackup> {
    let _clip = Clipboard::new_attempts(10).context("failed to open clipboard for backup")?;
    let unicode_text = {
        let mut text = String::new();
        if formats::Unicode.read_clipboard(&mut text).is_ok() {
            Some(text)
        } else {
            None
        }
    };

    Ok(ClipboardBackup { unicode_text })
}

fn restore_clipboard(snapshot: &ClipboardBackup) -> anyhow::Result<()> {
    let _clip = Clipboard::new_attempts(10).context("failed to open clipboard for restore")?;
    raw::empty().context("failed to empty clipboard before restore")?;

    if let Some(text) = snapshot.unicode_text.as_deref() {
        let owned = text.to_string();
        formats::Unicode
            .write_clipboard(&owned)
            .context("failed to restore clipboard text")?;
    }

    Ok(())
}

fn wait_for_selection_text(
    sequence_before: Option<std::num::NonZeroU32>,
    previous_text: Option<&str>,
) -> anyhow::Result<String> {
    for _ in 0..14 {
        thread::sleep(Duration::from_millis(40));

        let _clip = Clipboard::new_attempts(10).context("failed to open clipboard for lookup")?;
        let sequence_after = raw::seq_num();
        let mut current_text = String::new();
        let has_text = formats::Unicode.read_clipboard(&mut current_text).is_ok();
        drop(_clip);

        if !has_text {
            continue;
        }

        let trimmed = current_text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let sequence_changed = sequence_after != sequence_before;
        let text_changed = previous_text.is_none_or(|previous| previous.trim() != trimmed);

        if sequence_changed || text_changed {
            return Ok(trimmed.to_string());
        }
    }

    Err(anyhow!("没能抓到选中文本，请确认文本仍保持选中状态"))
}

fn current_cursor_position() -> anyhow::Result<AnchorPoint> {
    let mut point = POINT::default();
    unsafe {
        GetCursorPos(&mut point)
            .ok()
            .context("failed to read cursor position")?;
    }

    Ok(AnchorPoint {
        x: point.x,
        y: point.y,
    })
}

fn trigger_copy_shortcut() -> anyhow::Result<()> {
    let ctrl_down = keyboard_input(VK_CONTROL, false);
    let c_down = keyboard_input(VIRTUAL_KEY(0x43), false);
    let c_up = keyboard_input(VIRTUAL_KEY(0x43), true);
    let ctrl_up = keyboard_input(VK_CONTROL, true);
    let inputs = [ctrl_down, c_down, c_up, ctrl_up];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(anyhow!("failed to trigger copy shortcut"));
    }

    Ok(())
}

fn trigger_copy_insert_shortcut() -> anyhow::Result<()> {
    let ctrl_down = keyboard_input(VK_CONTROL, false);
    let insert_down = keyboard_input(VK_INSERT, false);
    let insert_up = keyboard_input(VK_INSERT, true);
    let ctrl_up = keyboard_input(VK_CONTROL, true);
    let inputs = [ctrl_down, insert_down, insert_up, ctrl_up];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(anyhow!("failed to trigger copy shortcut"));
    }

    Ok(())
}

fn keyboard_input(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
