use std::{ffi::c_void, sync::{OnceLock, RwLock, RwLockWriteGuard}};

use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::{WM_HOTKEY, MSG}};


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JGlobalHotkey {
    id: usize,
}

#[derive(Debug)]
pub enum JGlobalHotkeyErrors {
    NoKeySpecified,
    InvalidKey(char),
    InvalidModifier(String),
    RegisterFailed
}


impl JGlobalHotkey {

    pub fn is_valid(&self) -> bool {
        return  self.id > 0;
    }

    pub fn modifier_shift(&self) -> bool {
        self.id & 1 << 17 > 0
    }

    pub fn modifier_alt(&self) -> bool {
        self.id & 1 << 18 > 0
    }

    pub fn modifier_ctrl(&self) -> bool {
        self.id & 1 << 19 > 0
    }

    fn vk_code(&self) -> i16 {
        self.id as i16
    }

    fn modifiers(&self) -> HOT_KEY_MODIFIERS {
        let mut modifiers: HOT_KEY_MODIFIERS = MOD_NOREPEAT;
        if self.modifier_shift() {modifiers |= MOD_SHIFT;}
        if self.modifier_alt() {modifiers |= MOD_ALT;}
        if self.modifier_ctrl() {modifiers |= MOD_CONTROL;}
        modifiers
    }
    
    pub fn from_str(repr: &str) -> Result<JGlobalHotkey, JGlobalHotkeyErrors> {
        let my_string: String = repr.chars().filter(|&x| x != ' ').map(|c| c.to_ascii_lowercase()).collect();
        let codes: Vec<&str> = my_string.split('+').collect();
        let key = codes.last().ok_or(JGlobalHotkeyErrors::NoKeySpecified)?;
        let vk_code = match key.chars().count() {
            0 => Err(JGlobalHotkeyErrors::NoKeySpecified),
            1 => unsafe {
                let c = key.chars().last().unwrap();
                match VkKeyScanW(c as u16) {
                    -1 => Err(JGlobalHotkeyErrors::InvalidKey(c)),
                    res => Ok(res)
                }
            }
            _ => panic!("Not implemented")
        }?;

        let mut good_size_id = vk_code as i32;
        // no repeat
        good_size_id |= 1 << 20;
        for val in codes[..codes.len() - 1].iter() {
            match val {
                &"shift" => {
                    good_size_id |= 1 << 17;
                }
                &"alt" => {
                    good_size_id |= 1 << 18;
                }
                &"ctrl" => {
                    good_size_id |= 1 << 19;
                }
                _ => {
                    return Err(JGlobalHotkeyErrors::InvalidModifier(String::from(*val)));
                }
            }
        };
        Ok(JGlobalHotkey { id: good_size_id as _ })
    }




}

impl TryFrom<&str> for JGlobalHotkey {

    type Error = JGlobalHotkeyErrors;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        JGlobalHotkey::from_str(value)
    }
}


impl Default for JGlobalHotkey {

    fn default() -> Self {
        JGlobalHotkey { id: 0 }
    }
}


struct JGlobalHotKeyManagerInner {
    current_hotkey_down: Option<JGlobalHotkey>,
    hotkey_just_pressed: bool
}

impl Default for JGlobalHotKeyManagerInner {
    fn default() -> Self {
        JGlobalHotKeyManagerInner {
            current_hotkey_down: None,
            hotkey_just_pressed: false
        }
    }
}


pub enum JGlobalHotKeyEvent {
    None,
    HotkeyDown(JGlobalHotkey),
    HotkeyPressed(JGlobalHotkey),
    HotkeyReleased(JGlobalHotkey)
}


pub struct JGlobalHotkeyManager {
    inner: RwLock<JGlobalHotKeyManagerInner>
}


static GLOBAL_HOKTEY_MANAGER: OnceLock<JGlobalHotkeyManager> = OnceLock::new();


impl JGlobalHotkeyManager {

    fn init() -> Self {
        Self { inner: RwLock::new(JGlobalHotKeyManagerInner::default()) }
    }

    pub fn register(hotkey: &JGlobalHotkey) -> Result<&Self, JGlobalHotkeyErrors> {
        let out = GLOBAL_HOKTEY_MANAGER.get_or_init(|| JGlobalHotkeyManager::init() );
        match unsafe { 
            RegisterHotKey(
                0,
                hotkey.id as _,
                hotkey.modifiers(),
                hotkey.vk_code() as _
            )
        } {
            0 => Err(JGlobalHotkeyErrors::RegisterFailed),
            _ => Ok(out)
        }
    }

    #[inline]
    fn inner_mut() -> Option<RwLockWriteGuard<'static, JGlobalHotKeyManagerInner>> {
        if let Some(mananger ) = GLOBAL_HOKTEY_MANAGER.get() {
            return match mananger.inner.try_write() {
                Ok(v) => Some(v),
                _ => None
            }
        }
        None
    }

    // fn inner() -> Option<&'static GlobalHotKeyManagerInner> {
    //     if let Some(lock) = GLOBAL_HOKTEY_MANAGER.get() {
    //         lock.read()
    //     }
    //     None
    // }

    pub fn process_msg(msg: *const c_void) -> bool {
        if let Some(mut inner) = JGlobalHotkeyManager::inner_mut() {
            let msg = msg as *const MSG;
            unsafe {
                // https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-hotkey
                if (*msg).message == WM_HOTKEY {
                    inner.current_hotkey_down = Some(JGlobalHotkey { id: (*msg).wParam });
                    inner.hotkey_just_pressed = true;
                    return true;
                }
            };
        }
        false
    }

    pub fn event() -> JGlobalHotKeyEvent {
        if let Some(mut inner) = JGlobalHotkeyManager::inner_mut() {
            if inner.hotkey_just_pressed {
                inner.hotkey_just_pressed = false;
                return JGlobalHotKeyEvent::HotkeyPressed(inner.current_hotkey_down.unwrap());
            }
            if let Some(hotkey) = inner.current_hotkey_down {
                if unsafe { GetAsyncKeyState (hotkey.vk_code() as _) } == 0 {
                    let out = inner.current_hotkey_down.unwrap();
                    inner.current_hotkey_down = None;
                    return JGlobalHotKeyEvent::HotkeyReleased(out);

                }
            }
        }
        JGlobalHotKeyEvent::None
    }


}



#[cfg(test)]
mod test {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VkKeyScanW, MOD_ALT, MOD_NOREPEAT};

    use crate::JGlobalHotkey;

    #[test]
    fn vk_code_test() {
        let code = unsafe { VkKeyScanW('p' as _) };

        let gkey = JGlobalHotkey::try_from("Alt+P").unwrap();
        
        assert_eq!(code, gkey.vk_code());
    }
    #[test]
    fn modifier_test() {
        let gkey = JGlobalHotkey::from_str("Alt+P").unwrap();
        assert_eq!(gkey.modifiers(), MOD_ALT | MOD_NOREPEAT)
    }
}
