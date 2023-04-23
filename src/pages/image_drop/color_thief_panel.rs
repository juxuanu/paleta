/* color_thief_panel.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::{prelude::*, subclass::prelude::*};
use gtk::{glib, glib::clone, glib::Receiver, glib::Sender, CompositeTemplate, gio::ListStore};

use std::{cell::{RefCell, Cell}, thread};
use color_thief::{get_palette, ColorFormat};
use log::debug;

use crate::dialog::save_palette_dialog::SavePaletteDialog;
use crate::toasts::add_error_toast;
use crate::i18n::i18n;

use super::dropped_image::DroppedImage;
use super::extracted_color::ExtractedColor;
use super::extracted_color_card::ExtractedColorCard;

#[derive(Clone, Debug)]
pub enum ExtractionAction {
    ExtractedColors(Option<Vec<(u8, u8, u8)>>),
    Test(String),
    Error
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Paleta/color_thief_panel.ui")]
    pub struct ColorThiefPanelPriv {
        #[template_child(id = "count_amount_spin")]
        pub count_amount_spin: TemplateChild<gtk::SpinButton>,

        #[template_child(id = "accuracy_row")]
        pub accuracy_row: TemplateChild<adw::ComboRow>,

        #[template_child(id = "palette_box")]
        pub palette_box: TemplateChild<gtk::Box>,

        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<gtk::Spinner>,

        #[template_child(id = "colors_flow_box")]
        pub colors_flow_box: TemplateChild<gtk::FlowBox>,

        #[template_child(id = "save_button")]
        pub save_button: TemplateChild<gtk::Button>,

        pub list_store: ListStore,
        
        pub count_amount: Cell<f64>,
        pub quality: Cell<u8>,
        

        pub image: RefCell<Option<DroppedImage>>,
        pub image_uri: RefCell<String>,

        pub sender: RefCell<Option<Sender<ExtractionAction>>>,
        pub receiver: RefCell<Option<Receiver<ExtractionAction>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ColorThiefPanelPriv {
        const NAME: &'static str = "ColorThiefPanel";
        type Type = super::ColorThiefPanel;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            Self {
                count_amount_spin: TemplateChild::default(),
                accuracy_row: TemplateChild::default(),
                palette_box: TemplateChild::default(),
                spinner: TemplateChild::default(),
                colors_flow_box: TemplateChild::default(),
                save_button: TemplateChild::default(),
                list_store: ListStore::new(ExtractedColor::static_type()),
                count_amount: Cell::new(0.0),
                quality: Cell::new(0),
                image: RefCell::new(None),
                image_uri: RefCell::new(String::new()),
                sender: RefCell::new(Some(sender)),
                receiver: RefCell::new(Some(r)),
            }
        }

    }

    impl ObjectImpl for ColorThiefPanelPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for ColorThiefPanelPriv {}
    impl BinImpl for ColorThiefPanelPriv {}
    impl ColorThiefPanelPriv {}
}

glib::wrapper! {
    pub struct ColorThiefPanel(ObjectSubclass<imp::ColorThiefPanelPriv>)
    @extends gtk::Widget, adw::Bin;
}

impl ColorThiefPanel {
    pub fn new() -> ColorThiefPanel {
        let color_panel: ColorThiefPanel = glib::Object::builder::<ColorThiefPanel>().build();
        color_panel
    }

    fn initialize(&self) {
        let imp = self.imp();

        imp.colors_flow_box.bind_model(
            Some(&imp.list_store), 
        clone!(@strong self as this => @default-panic, move |obj| {
                let color = obj.clone().downcast::<ExtractedColor>().expect("ExtractedColor is of wrong type");       
                ExtractedColorCard::new(&color).upcast::<gtk::Widget>()
            })
        );

        // imp.colors_flow_box.connect_row_selected(clone!(@strong self as this => @default-panic, move |_listbox, obj| {
        //     match obj {
        //         Some(row) => {
        //             let ec_row = row.clone().downcast::<ExtractedColorRow>().expect("ExtractedColorRow is of wrong type");    
        //             let hex_name = ec_row.hex_name();
        //             copy_color(hex_name);
        //         },
        //         None => (),
        //     }
        // }));

        imp.save_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                this.save_palette();
            })
        );

        imp.accuracy_row.set_selected(1);

        imp.accuracy_row.set_selected(1);
        imp.count_amount.set(imp.count_amount_spin.value());
        imp.quality.set(self.quality());

        imp.count_amount_spin.connect_value_changed(
            clone!(@strong self as this => @default-panic, move |spin_button| {
                this.imp().count_amount.set(spin_button.value());
                this.start_extraction()
            })
        );
        
        imp.accuracy_row.connect_selected_notify(
            clone!(@strong self as this => @default-panic, move |_drop_down| {
                this.imp().quality.set(this.quality());
                this.start_extraction()
            })
        );

        self.setup_channel();
    }

    fn setup_channel(&self) {
        let imp = self.imp();
        let receiver = imp.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.clone().process_action(action)),
        );
    }

    fn quality(&self) -> u8 {
        match self.imp().accuracy_row.selected() {
            0 => return 1,
            1 => return 3,
            2 => return 10,
            _ => return 10,
        }
    }

    fn process_action(&self, action: ExtractionAction) -> glib::Continue {
        match action {
            ExtractionAction::ExtractedColors(colors) => self.extraction_done(colors),
            _ => debug!("Received action {:?}", action),
        }

        glib::Continue(true)
    }

    pub fn start_extraction(&self) {
        let imp = self.imp();
        if let Some(_image) = imp.image.borrow().as_ref() {
            imp.palette_box.set_visible(false);
            imp.spinner.set_visible(true);
            imp.spinner.start();
            imp.save_button.set_icon_name("star-outline-rounded-symbolic");
            let sender = imp.sender.borrow().as_ref().unwrap().clone();
            let pixbuf_bytes = imp.image.borrow().as_ref().unwrap().imp().pixbuf.borrow().as_ref().unwrap().clone().pixel_bytes().unwrap();
            let alpha = color_format(imp.image.borrow().as_ref().unwrap().imp().pixbuf.borrow().as_ref().unwrap().has_alpha());
            let quality = imp.quality.get() as u8;
            let count = imp.count_amount.get() as u8;
            thread::spawn(move || {
                let palette = load_palette_from_bytes(pixbuf_bytes.as_ref(), alpha, count, quality);
                let _ = sender.send(ExtractionAction::ExtractedColors(palette));
            });
        } else {
            add_error_toast(i18n("Unable to start palette extraction, no image loaded."));
        }
    }

    fn extraction_done(&self, colors: Option<Vec<(u8, u8, u8)>>) {
        if let Some(colors) = colors {
            let imp = self.imp();
            imp.spinner.stop();
            imp.spinner.set_visible(false);
            imp.palette_box.set_visible(true);
            imp.list_store.remove_all();

            for rgba in colors {
                imp.list_store.append(&ExtractedColor::new(rgba))
            }
        } else  {
            add_error_toast(i18n("Unable to extract colors from image."));

        }
        debug!("extraction_done");
    }

    fn save_palette(&self) {
        let imp = self.imp();
        if let Some(_image) = imp.image.borrow().as_ref() {
            if imp.list_store.n_items() > 0 {
                debug!("save palette");
            
                let mut colors = Vec::new(); 
                for position in 0..imp.list_store.n_items() {
                    let color = imp.list_store.item(position).unwrap().clone().downcast::<ExtractedColor>().expect("ExtractedColor is of wrong type");
                    colors.push(color);
                }

                let dialog = SavePaletteDialog::new(colors);

                dialog.connect_local(
                    "success",
                    false,
                    clone!(@weak self as this => @default-return None, move |_args| {
                        this.imp().save_button.set_icon_name("star-filled-rounded-symbolic");
                        None
                    }),
                );

                dialog.show();
            } else {
                add_error_toast(i18n("Unable to save palette, no colors extracted."));
            }
        } else {
            add_error_toast(i18n("Unable to save palette, no image loaded."));
        }
    }
    
    pub fn list_store(&self) -> &ListStore {
        &self.imp().list_store
    }
}

fn color_format(has_alpha: bool) -> ColorFormat {
    if has_alpha {
        ColorFormat::Rgba
    } else {
        ColorFormat::Rgb
    }
}

pub fn load_palette_from_bytes(pixbuf_bytes: &[u8], alpha: ColorFormat, count: u8, quality: u8) -> Option<Vec<(u8, u8, u8)>> {
    if let Ok(palette) = get_palette(
        pixbuf_bytes,
        alpha,
        quality,
        count,
    ) {
        let colors: Vec<(u8, u8, u8)> = palette
            .iter()
            .map(|c| {
                (                   
                    c.r,
                    c.g,
                    c.b,
                )

            })
            .collect();
        return Some(colors);
    }
    None
}