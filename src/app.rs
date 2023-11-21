use std::rc::Rc;
use std::{mem::size_of, sync::mpsc::Sender};

use std::sync::mpsc::{channel, Receiver};

use cpal::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use serde::{Serialize, Deserialize};

// use keyboard_types::{Code, Modifiers};
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::GetMessageExtraInfo};

use crate::{JGlobalHotkey, JGlobalHotkeyErrors, JGlobalHotkeyManager, JGlobalHotKeyEvent};
// use winit::window::Window;

#[derive(Debug)]
struct SoundDataIter {
    current: *const f32,
    end: *const f32
}

unsafe impl Send for SoundDataIter {}
unsafe impl Sync for SoundDataIter {}

impl SoundDataIter {
    
    fn new(sound_data: &Vec<f32>) -> SoundDataIter {
        let range = sound_data.as_ptr_range();
        SoundDataIter {
            current: range.start as *mut f32,
            end: range.end
        }
    }

    #[inline]
    fn next_value(&mut self) -> f32 {
        let value = unsafe { *self.current };
        if self.current != self.end { unsafe { self.current = self.current.add(1);}}
        return value;
    }

    #[inline]
    fn is_done(&self) -> bool {
        self.current == self.end
    }
}




trait JSystemInput {

    fn get_press_input(&self) -> INPUT;

    fn get_release_input(&self) -> INPUT;
}


pub enum JMouseButton {
    MouseButton1,
    MouseButton2,
    MouseButton3,
    MouseButton4,
    MouseButton5
}


impl JSystemInput for JMouseButton{

    fn get_press_input(&self) -> INPUT {
        let (event, data) = match self {
            JMouseButton::MouseButton1 => (MOUSEEVENTF_LEFTDOWN, 0),
            JMouseButton::MouseButton2 => (MOUSEEVENTF_RIGHTDOWN, 0),
            JMouseButton::MouseButton3 => (MOUSEEVENTF_MIDDLEDOWN, 0),
            JMouseButton::MouseButton4 => (MOUSEEVENTF_XDOWN, 1),
            JMouseButton::MouseButton5 => (MOUSEEVENTF_XDOWN, 2),
            
        };
        unsafe {
            let mouse_input = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: event,
                time: 0,
                dwExtraInfo: GetMessageExtraInfo() as _ 
            };
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 { mi: mouse_input }
            }
        }
    }

    fn get_release_input(&self) -> INPUT {
        let (event, data) = match self {
            JMouseButton::MouseButton1 => (MOUSEEVENTF_LEFTUP, 0),
            JMouseButton::MouseButton2 => (MOUSEEVENTF_RIGHTUP, 0),
            JMouseButton::MouseButton3 => (MOUSEEVENTF_MIDDLEUP, 0),
            JMouseButton::MouseButton4 => (MOUSEEVENTF_XUP, 1),
            JMouseButton::MouseButton5 => (MOUSEEVENTF_XUP, 2),
            
        };
        unsafe {
            let mouse_input = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: event,
                time: 0,
                dwExtraInfo: GetMessageExtraInfo() as _ 
            };
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 { mi: mouse_input }
            }
        }

    }
    
}

pub enum JAppEvent {
    StopAudio
}

#[derive(Serialize, Deserialize)]
pub struct JAppState {
    pub current_device: Option<String>,
    pub stop_audio_on_release: bool
}


impl Default for JAppState {

    fn default() -> Self {
        JAppState { 
            current_device: None,
            stop_audio_on_release: false
        }
   } 
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundId(usize);


struct Sound {
    data: Vec<f32>,
    // duration: std::time::Duration,
    path: String,
    sound_id: SoundId
}

pub struct JApp {
    simulate_key_press_on_play: Vec<Box<dyn JSystemInput>>,
    // main_window: Option<Window>,
    sender: Sender<JAppEvent>,
    receiver: Receiver<JAppEvent>,
    pub state: JAppState,
    sound_stream: Option<Rc<cpal::Stream>>,
    sounds: Vec<Sound>,
    sound_idx_count: usize,
    hotkey_sound_mapping: Vec<(JGlobalHotkey, SoundId)>
}

// https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/virtual-audio-devices
impl JApp {

    pub fn new() -> Self {
        let (jtx, jrx) = channel::<JAppEvent>();
        JApp { 
            simulate_key_press_on_play: Vec::new(),
            // main_window: None,
            sender: jtx,
            receiver: jrx,
            state: JAppState::default(),
            sound_stream: None,
            sounds: Vec::new(),
            sound_idx_count: 0,
            hotkey_sound_mapping: Vec::new()
        }
    }

    pub fn register_hoktey_for_sound(&mut self, hotkey: &str, sound_id: SoundId) -> Result<(), String> {
        match JGlobalHotkey::try_from(hotkey) {
            Ok(hotkey) => {
                match JGlobalHotkeyManager::register(&hotkey) {
                    Ok(_) => {
                        self.hotkey_sound_mapping.push((hotkey, sound_id));
                        Ok(())
                    },
                    Err(_) => Err(String::from("Register failed."))
                }
            },
            Err(JGlobalHotkeyErrors::NoKeySpecified) => Err(String::from("No key specified")),
            Err(JGlobalHotkeyErrors::RegisterFailed) => Err(String::from("Couldn't register hotkey")),
            Err(JGlobalHotkeyErrors::InvalidKey(k)) => Err(format!("Invalid key {k}")),
            Err(JGlobalHotkeyErrors::InvalidModifier(k)) => Err(format!("Invalid modifiers{k}"))
        }
    }

    pub fn load_sound(&mut self, path: &str) -> Option<SoundId> {
        let mut inp_file = std::fs::File::open(std::path::Path::new(path)).unwrap();
        let (_, data) = wav::read(&mut inp_file).unwrap();
        match data {
            wav::BitDepth::ThirtyTwoFloat(v) => {
                // let len = (v.len() as f64 / header.channel_count as f64) / header.sampling_rate as f64;
                // duration: std::time::Duration::from_millis((len * 1000.0) as _),
                let sound_id = SoundId(self.sound_idx_count);
                self.sound_idx_count += 1;
                self.sounds.push(
                    Sound { 
                        data: v,
                        path: String::from(path),
                        sound_id
                    }
                );
                return Some(sound_id);
            },
            _ => ()
        };
        None
    }

    pub fn load_sate(&mut self) {
        match std::fs::read(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/app_state.json"))) {
            Ok(data) => {
               match serde_json::from_slice::<JAppState>(data.as_slice()) {
                Ok(state) => self.state = state,
                Err(e) => println!("Couldn't load state {e:?}")
               }
            }, 
            Err(e) => println!("Couldn't load state: {e:?}")
        }
        println!("Using output device: {}", self.state.current_device.as_ref().unwrap_or(&String::new()));
    }

    pub fn save_state(&self) {
        if let Err(e) = std::fs::write(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/app_state.json")),
            serde_json::to_string(&self.state).unwrap()
        ) {
            println!("Error saving app sate: {e:?}");
        }
    }

    pub fn list_input_device() -> Vec<String> {
       let host = cpal::default_host();
       if let Ok(devices) = host.input_devices() {
            return devices.filter_map(|d| d.name().ok()).collect();
       }
       Vec::new()
    }

    pub fn get_output_audio_device(&self) -> Option<Device> {
        if let Some(target_name) = self.state.current_device.as_ref() {
            if let Ok(devices) = cpal::default_host().output_devices() {
                for d in devices {
                    if let Ok(name) = d.name() {
                        if &name == target_name {
                            return  Some(d);
                        }
                    }
                }
            }
        }
        return cpal::default_host().default_output_device();
    }

    pub fn get_ouptut_audio_devices() -> Vec<String>{
        let host = cpal::default_host();
        if let Ok(devices) = host.output_devices() {
            return devices.filter_map(|d| d.name().ok()).collect();
        }
        Vec::new()
    }

    pub fn set_output_device(&mut self, device_name: &str) {
        self.state.current_device = Some(String::from(device_name));
    }

    pub fn with_mouse_input_on_play(&mut self, mouse_input: JMouseButton) -> &Self {
        self.simulate_key_press_on_play.push(Box::new(mouse_input));
        self
    }

    fn get_sound(&self, sound_id: SoundId) -> Option<&Sound> {
        for sound in self.sounds.iter() {
            if sound.sound_id == sound_id {
                return Some(sound);
            }
        }
        None
    }

    fn get_sound_player(&self, sound_id: SoundId) -> Option<SoundDataIter> {
        let sound = self.get_sound(sound_id)?;
        Some(SoundDataIter::new(&sound.data))
    }

    pub fn play(&mut self, sound_id: SoundId) {
        self.stop();

        let sound_player = match self.get_sound_player(sound_id) {
            Some(s) => s,
            _ => return
        };

        let inputs = self.simulate_key_press_on_play
            .iter()
            .map(|i| i.get_press_input())
            .collect::<Vec<INPUT>>();
        if inputs.len() > 0 {
            unsafe {
                SendInput(inputs.len() as _, inputs.as_ptr(), size_of::<INPUT>() as _);
            }
        };
        self.play_sound(sound_player);
    }

    pub fn stop(&mut self) {
        if let Some(_) = self.sound_stream.take() {
            // dropping the stream should stop it.
            let inputs = self.simulate_key_press_on_play
            .iter()
            .map(|i| i.get_release_input())
            .collect::<Vec<INPUT>>();
            if inputs.len() > 0 {
                unsafe {
                    SendInput(inputs.len() as _, inputs.as_ptr(), size_of::<INPUT>() as _);
                }
            };
        };
    }

    // #[allow(dead_code)] 
    // fn play_sound(&self) {
    //     if let Some(device) = self.get_output_audio_device(){
    //         println!("Using output device: {}", device.name().unwrap());
    //         let config = device.default_output_config().unwrap();
    //         println!("Sample format: {:?}", config.sample_format());
    //         println!("Sample rate: {:?}", config.sample_rate());
    //         println!("Channel: {:?}", config.channels());
    //         std::thread::spawn(move || {
    //             // see cpal example for more format info
    //             match config.sample_format() {
    //                 cpal::SampleFormat::F32 => play_sound_open::<f32>(&device, &config.into()),
    //                 sample_format => panic!("Unsupported sample format '{sample_format}'"),
    //             };
    //         });
    //     } else {
    //         println!("Error couldn't find audio device.");
    //     }
    // }

    fn play_sound(&mut self, mut sound_player: SoundDataIter) {
        if let Some(device) = self.get_output_audio_device(){
            let config = device.default_output_config().unwrap();
            // maybe make sure that this is two channels ?
            // let channels = config.channels() as usize;

            // TODO: fix unwrap
            match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let stop_sender = self.sender.clone();
                    match device.build_output_stream(
                        &config.into(),
                        move |output: &mut[f32], _info| {
                            // iterator over [0, 1 ... nChannel] blocks
                            for sample in output.iter_mut() {
                                // set the value of each channel
                                *sample = sound_player.next_value();
                            }
                            if sound_player.is_done() {
                                let _ = stop_sender.send(JAppEvent::StopAudio);
                            }
                        },
                        |err| eprintln!("an error occurred on stream: {}", err),
                        None
                    ) {
                        Ok(stream) => {
                            if let Ok(_) = stream.play() {
                                self.sound_stream = Some(Rc::new(stream));
                            }
                        },
                        Err(e) => {
                            println!("Error building stream {e} :(");
                        }
                    };
                },
                sample_format => panic!("Unsupported sample format '{sample_format}'"),
            };
        }
    }

    pub fn process_events(&mut self) -> bool {
        match JGlobalHotkeyManager::event() {
            JGlobalHotKeyEvent::HotkeyPressed(ref hotkey) => {
                for (shk, sound_id) in self.hotkey_sound_mapping.iter() {
                    if shk == hotkey {
                        self.play(*sound_id);
                        return true;
                    }
                }
            },
            JGlobalHotKeyEvent::HotkeyReleased(ref hotkey) => {
                if self.state.stop_audio_on_release {
                    for (shk, _) in self.hotkey_sound_mapping.iter() {
                        if shk == hotkey {
                            self.stop();
                            return true;
                        }
                    }
                }
            }
            _ => ()
        }

        loop {
            if let Ok(event) = self.receiver.try_recv() {
                match event {
                    JAppEvent::StopAudio => {
                        self.stop();
                        return true;
                    }
                }
            } else {
                return false;
            }
        }
    }

}

