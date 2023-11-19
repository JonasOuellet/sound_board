use slint::ComponentHandle;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder
};

use winittray::{
    JGlobalHotkeyManager,
    JApp,
    JMouseButton,
    JAppUI
};

use image;


struct TrayMenu {
    show_window_item: MenuItem,
    close_item: MenuItem,
}

enum TrayAction {
    ShowWindow,
    Close,
    Invalid
}

impl TrayMenu {
    
    fn new() -> Self {
        let show_window_item = MenuItem::new("Show Window", true, None);
        let close_item = MenuItem::new("Close", true, None);
        TrayMenu {
            show_window_item,
            close_item,
        }
    }
    
    fn build(&self) -> Box<Menu> {
        let menu = Box::new(Menu::new());
        menu.append(&self.show_window_item).unwrap();
        menu.append(&self.close_item).unwrap();
        menu
    }

    fn action_from_event(&self, event: &MenuEvent) -> TrayAction {
        let id = event.id();
        if id == self.show_window_item.id() {
            return TrayAction::ShowWindow;
        }

        if id == self.close_item.id() {
            return TrayAction::Close;
        } 
        
        return TrayAction::Invalid;
    }

}


fn main() {
    let mut app = JApp::new();
    app.load_sate();
    app.with_mouse_input_on_play(JMouseButton::MouseButton5);

    let sound_id = app.load_sound(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/chiguibigoula.wav")).unwrap();
    app.register_hoktey_for_sound("ALT+P", sound_id).unwrap();

    let sound_id = app.load_sound(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/wiggle.wav")).unwrap();
    app.register_hoktey_for_sound("ALT+O", sound_id).unwrap();

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/icon.png");
    let icon = load_icon(std::path::Path::new(path));

    let tray_menu = TrayMenu::new();
    #[allow(unused)]
    let tray_icon = TrayIconBuilder::new()
        .with_menu(tray_menu.build())
        .with_tooltip("winit - awesome windowing lib")
        .with_icon(icon)
        .build()
        .unwrap();

    let menu_channel = MenuEvent::receiver();
    // let tray_channel = TrayIconEvent::receiver();
    let mut window: Option<JAppUI> = None;
    let backend = i_slint_backend_winit::BackendBuilder::new()
        .with_quit_on_last_window(false)
        .with_msg_hook(|msg| JGlobalHotkeyManager::process_msg(msg))
        .with_pre_callback(move |_, _| 
            {
                if app.process_events() {
                    return true;
                }

                if let Ok(event) = menu_channel.try_recv() {
                    match tray_menu.action_from_event(&event) {
                        TrayAction::ShowWindow => {
                            match window.as_mut() {
                                Some(ui) => ui.show().unwrap(),
                                None => {
                                    let app = JAppUI::new().unwrap();
                                    app.show().unwrap();
                                    window = Some(app);
                                }
                            }
                        },
                        TrayAction::Close => {
                            if let Some(win) = window.as_mut() {
                                win.hide().unwrap();
                            }
                        }
                        _ => ()
                    }
                }
                
                return false;
            }
        )
        .with_post_callback(|event_target|
            {
                // making sure
                event_target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            }
        )
        .build()
        .unwrap();
    slint::platform::set_platform(Box::new(backend)).unwrap();
    slint::run_event_loop().unwrap();
}


fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    // tray_icon::Icon::from_resource(ordinal, size)
}