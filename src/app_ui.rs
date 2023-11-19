slint::slint!{

    export global JAppLogic {
        in property <[{name: string, id: int}]> output_devices;
    }

    component Empty {
    }

    component Sound {
        in property <bool> selected;
        in property <string> name <=> txt.text;
        height: touch-area.has-hover ? 44px : 40px;
        animate height {
            duration: 100ms; 
        }
        HorizontalLayout {
            padding: touch-area.has-hover ? 0px: 4px;
            animate padding {
                duration: 100ms; 
            }
            rec := Rectangle {
                border-radius: 6px;
                background: touch-area.has-hover ? #04b5eb : #0087b1;
                animate background {
                    duration: 100ms; 
                }
                clip: true;
                drop-shadow-color: self.background.darker(100%);
                drop-shadow-offset-x: 3px;
                drop-shadow-offset-y: 3px;
                txt := Text {
                    min-width: 10px;
                    font-size: 18px;
                    font-weight: 800;
                    color: #ffffff;
                    overflow: elide;
                    wrap: no-wrap;
                    // font-weight: 5;
                    // font-family: ;
                }
                touch_area := TouchArea {
                    clicked => {}
                }
            }
        }
    }

    component SoundDevice {
        Rectangle {
            clip: true;
            border-radius: 10px;
            background: #033442;
            VerticalLayout {
                padding-top: 20px;
                padding-bottom: 20px;
                padding-left: 5px;
                padding-right: 5px;
                spacing: 8px;
                for data in JAppLogic.output-devices: rep := Sound {
                    name: data.name;
                }
                Sound {
                    name: "realtek audio";
                }
                Sound {
                    name: "Ecran de droite";
                }
                Empty {}
            }
        } 
    }
    export component JAppUI inherits Window {
        HorizontalLayout {
            SoundDevice {

            }
        }
    }
}



#[cfg(test)]
mod test {
    use std::rc::Rc;

    use i_slint_backend_winit::Backend;
    use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

    use super::{JAppUI, JAppLogic};

    #[test]
    fn set_devices() {
        slint::platform::set_platform(Box::new(Backend::new().unwrap())).unwrap();
        let app = JAppUI::new().unwrap();
        let vec: Vec<(i32, SharedString)> = vec![(1, "Test1".into()), (2, "Test".into())];
        let model: Rc<VecModel<(i32, SharedString)>> = Rc::new(VecModel::from(vec));
        let model_rc = ModelRc::from(model.clone());
        app.global::<JAppLogic>().set_output_devices(model_rc);
        model.push((3, "allo".into()));
        app.run().unwrap();
    }

}