
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

pub mod ui {
    slint::include_modules!();
}

use ui::*;

pub struct JAppUI {
    ui: Option<JAppUISlint>
}


impl JAppUI {

    pub fn new() -> Self{
        JAppUI {
            ui: None
        }
    }

    pub fn show(&mut self) -> bool {
        if let Some(app) = self.ui.as_ref() {
            let _ = app.show().unwrap();
            return false;
        }
        // init the app.  asign it and show it.
        let app = JAppUISlint::new().unwrap();
        app.show().unwrap();
        self.ui = Some(app);
        return true;
    }

    pub fn set_devices(&self, devices: &[String]) {
        if let Some(app) = self.ui.as_ref() {
            let vec: Vec<SharedString> = devices.iter().map(|v| SharedString::from(v)).collect();
            let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(vec));
            let model_rc = ModelRc::from(model.clone());
            app.set_output_devices(model_rc);
        }
    }
    
    pub fn set_current_device(&self, device: &String) {
        if let Some(app) = self.ui.as_ref() {
            app.set_current_output_device(device.into());
        }
    }
}
