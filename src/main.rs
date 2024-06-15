#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, sleep},
    time::{Duration, Instant},
};

use eframe::egui::{self, Align2, Color32, IconData, Rect, RichText, Rounding, Sense};
use image::GenericImageView;
use inputbot::KeybdKey;
use mouse_rs::Mouse;
use rand::Rng;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

#[derive(AsRefStr, Eq, PartialEq, EnumIter, Clone, Copy, Debug)]
enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(AsRefStr, Eq, PartialEq, EnumIter, Clone, Copy, Debug)]
enum ClickMode {
    Click,
    Toggle,
}

#[derive(AsRefStr, PartialEq, EnumIter, Clone, Copy, Debug)]
enum LimitMode {
    None,
    Clicks,
    Time,
}

#[derive(AsRefStr, PartialEq, EnumIter, Clone, Copy, Debug)]
enum IntervalMode {
    Constant,
    Random,
}

fn percentage_distance_between_colors(a: Color32, b: Color32) -> f32 {
    let distance_r = a.r().abs_diff(b.r()) as f32;
    let distance_g = a.g().abs_diff(b.g()) as f32;
    let distance_b = a.b().abs_diff(b.b()) as f32;

    let distance = (distance_r.powi(2) + distance_g.powi(2) + distance_b.powi(2)).sqrt();
    let percentage = distance / 441.672956;

    percentage
}

fn main() -> Result<(), eframe::Error> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory_with_format(
            include_bytes!("./assets/Click.png"),
            image::ImageFormat::Png,
        )
        .unwrap();
        let (width, height) = image.dimensions();
        let rgba = image.into_rgba8().into_vec();
        (rgba, width, height)
    };

    eframe::run_native(
        "Clickr",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 400.0])
                .with_always_on_top()
                .with_maximize_button(false)
                .with_active(true)
                .with_icon(IconData {
                    rgba: icon_rgba,
                    width: icon_width,
                    height: icon_height,
                })
                .with_resizable(false),
            ..Default::default()
        },
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(AppHolder::default())
        }),
    )
}

struct AppHolder {
    main_app: Arc<Mutex<App>>,
}

impl AppHolder {
    fn app(&self) -> MutexGuard<App> {
        self.main_app.lock().unwrap()
    }
    fn app_mut(&mut self) -> MutexGuard<App> {
        self.main_app.lock().unwrap()
    }

    fn click_loop(&mut self) {
        let mut app = self.app_mut();
        app.total_clicks = 0;
        app.mouse_is_pressed = false;
        app.clicker_id += 1;
        let clicker_id = app.clicker_id;
        drop(app);

        loop {
            let mut app = self.app_mut();
            if !app.clicker_enabled || clicker_id != app.clicker_id {
                break;
            }

            match app.limit_mode {
                LimitMode::Clicks => {
                    if app.total_clicks >= app.limit_mode_clicks_amount {
                        app.clicker_enabled = false;
                        break;
                    }
                }
                LimitMode::Time => {
                    if Instant::now()
                        .duration_since(app.clicker_start_time)
                        .as_secs_f32()
                        >= app.limit_mode_time
                    {
                        app.clicker_enabled = false;
                        break;
                    }
                }
                _ => {}
            }

            let should_click: bool = !app.color_mode
                || (app.color_mode
                    && percentage_distance_between_colors(
                        app.hovering_pixel_color,
                        app.color_mode_color,
                    ) <= app.color_mode_distance_threshold);

            if should_click {
                app.mouse_is_pressed = !app.mouse_is_pressed;
                app.click_mouse();
                app.total_clicks += 1;
            }

            let total_seconds: f64 = app.hours as f64 * 3600.0
                + app.minutes as f64 * 60.0
                + app.seconds as f64
                + app.milliseconds as f64 / 1000.0;

            let time_to_wait: f64 = match app.interval_mode {
                IntervalMode::Constant => total_seconds,
                IntervalMode::Random => {
                    let mut rng = rand::thread_rng();

                    rng.gen_range(
                        app.interval_mode_random_min as f64..=app.interval_mode_random_max as f64,
                    )
                }
            };

            drop(app);

            sleep(Duration::from_secs_f64(time_to_wait));
        }
    }

    fn start_clicker(&self) {
        let app_arc_clone = Arc::clone(&self.main_app);
        thread::spawn(move || {
            let mut holder = AppHolder {
                main_app: app_arc_clone,
            };
            holder.click_loop();
        });
    }

    fn toggle_clicker(&mut self) {
        let mut app = self.app_mut();
        app.clicker_enabled = !app.clicker_enabled;
    }
}

struct App {
    mouse: Mouse,

    interval_mode: IntervalMode,
    hours: u32,
    minutes: u32,
    seconds: u32,
    milliseconds: u32,

    interval_mode_random_min: f32,
    interval_mode_random_max: f32,

    mouse_button: MouseButton,
    click_mode: ClickMode,

    mouse_is_pressed: bool,

    clicker_id: u32,

    color_mode: bool,
    color_mode_color: Color32,
    color_mode_distance_threshold: f32,
    hovering_pixel_color: Color32,

    limit_mode: LimitMode,
    limit_mode_clicks_amount: u32,
    limit_mode_time: f32,

    clicker_enabled: bool,
    last_clicker_enabled: bool,
    clicker_start_time: Instant,
    total_clicks: u32,
}

impl Default for AppHolder {
    fn default() -> Self {
        let new_app = App {
            mouse: Mouse::new(),

            interval_mode: IntervalMode::Constant,
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 100,

            interval_mode_random_min: 1.0,
            interval_mode_random_max: 2.0,

            mouse_button: MouseButton::Left,
            click_mode: ClickMode::Click,

            mouse_is_pressed: false,

            clicker_id: 0,

            color_mode: false,
            color_mode_color: Color32::BLACK,
            hovering_pixel_color: Color32::BLACK,

            limit_mode: LimitMode::None,
            limit_mode_clicks_amount: 10,
            color_mode_distance_threshold: 0.0,
            limit_mode_time: 1.0,

            clicker_enabled: false,
            last_clicker_enabled: false,
            clicker_start_time: Instant::now(),
            total_clicks: 0,
        };

        let app_arc = Arc::new(Mutex::new(new_app));
        let app_arc_clone = app_arc.clone();

        KeybdKey::F6Key.bind(move || {
            let mut app = app_arc_clone.lock().unwrap();
            app.clicker_enabled = !app.clicker_enabled;
        });

        thread::spawn(|| inputbot::handle_input_events());

        AppHolder { main_app: app_arc }
    }
}

impl App {
    fn click_mouse(&self) {
        let button = match self.mouse_button {
            MouseButton::Left => mouse_rs::types::keys::Keys::LEFT,
            MouseButton::Middle => mouse_rs::types::keys::Keys::MIDDLE,
            MouseButton::Right => mouse_rs::types::keys::Keys::RIGHT,
        };

        match self.click_mode {
            ClickMode::Toggle => {
                if self.mouse_is_pressed {
                    self.mouse.press(&button).expect("Unable to press button");
                } else {
                    self.mouse
                        .release(&button)
                        .expect("Unable to release button");
                }
            }
            _ => self.mouse.click(&button).expect("Unable to click button"),
        }
    }
    fn try_release_mouse(&mut self) {
        if !self.mouse_is_pressed {
            return;
        };
        let button = match self.mouse_button {
            MouseButton::Left => mouse_rs::types::keys::Keys::LEFT,
            MouseButton::Middle => mouse_rs::types::keys::Keys::MIDDLE,
            MouseButton::Right => mouse_rs::types::keys::Keys::RIGHT,
        };

        self.mouse
            .release(&button)
            .expect("Unable to release button");
        self.mouse_is_pressed = false;
    }
}

impl eframe::App for AppHolder {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let app = self.app();
                let enabled = !app.clicker_enabled;
                drop(app);

                ui.add_enabled_ui(enabled, |ui| {
                    let mut app = self.app_mut();

                    egui::Frame::popup(&ctx.style()).show(ui, |ui| {
                        egui::Frame::popup(&ctx.style())
                            .fill(Color32::from_rgb(0, 170, 255))
                            .show(ui, |ui| {
                                let mut available = ui.available_rect_before_wrap();
                                available.set_height(0.0);
                                ui.allocate_rect(available, Sense::focusable_noninteractive());
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [20.0, 20.0],
                                        egui::Image::new(egui::include_image!(
                                            "./assets/ClickInterval.png"
                                        )),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new("Click Interval")
                                                .heading()
                                                .text_style(egui::TextStyle::Heading)
                                                .color(Color32::WHITE),
                                        )
                                        .selectable(false),
                                    );
                                });
                            });
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            if ui
                                .radio(app.interval_mode == IntervalMode::Constant, "Set Time")
                                .clicked()
                            {
                                app.interval_mode = IntervalMode::Constant;
                            }

                            ui.add_enabled_ui(app.interval_mode == IntervalMode::Constant, |ui| {
                                ui.columns(4, |columns| {
                                    let mut h = app.hours;
                                    let mut m = app.minutes;
                                    let mut s = app.seconds;
                                    let mut ms = app.milliseconds;

                                    let fields = [
                                        ("h", &mut h),
                                        ("m", &mut m),
                                        ("s", &mut s),
                                        ("ms", &mut ms),
                                    ];
                                    fields.into_iter().enumerate().for_each(
                                        |(i, (suffix, value))| {
                                            columns[i].push_id(i, |ui| {
                                                ui.add(
                                                    egui::DragValue::new(value)
                                                        .suffix(suffix)
                                                        .speed(1)
                                                        .max_decimals(0),
                                                )
                                            });
                                        },
                                    );

                                    app.hours = h;
                                    app.minutes = m;
                                    app.seconds = s;
                                    app.milliseconds = ms;
                                });
                            });

                            ui.add_space(15.0);

                            if ui
                                .radio(
                                    app.interval_mode == IntervalMode::Random,
                                    "Set Random Interval",
                                )
                                .clicked()
                            {
                                app.interval_mode = IntervalMode::Random;
                            }

                            ui.add_enabled_ui(app.interval_mode == IntervalMode::Random, |ui| {
                                ui.columns(2, |columns| {
                                    let mut min = app.interval_mode_random_min;
                                    let mut max = app.interval_mode_random_max;

                                    max = max.clamp(0.0, 3600.0);
                                    min = min.clamp(0.0, max);

                                    let fields = [&mut min, &mut max];
                                    fields.into_iter().enumerate().for_each(|(i, value)| {
                                        columns[i].push_id(i, |ui| {
                                            ui.add(
                                                egui::DragValue::new(value)
                                                    .suffix("s")
                                                    .speed(0.1)
                                                    .min_decimals(1)
                                                    .clamp_range(0..=3600)
                                                    .update_while_editing(false)
                                                    .max_decimals(3),
                                            );
                                        });
                                    });

                                    app.interval_mode_random_min = min;
                                    app.interval_mode_random_max = max;
                                });
                            });
                            ui.add_enabled_ui(app.interval_mode == IntervalMode::Random, |ui| {
                                ui.columns(2, |columns| {
                                    let fields = ["Min", "Max"];
                                    fields.into_iter().enumerate().for_each(|(i, text)| {
                                        columns[i].push_id(i, |ui| {
                                            ui.label(text);
                                        });
                                    });
                                });
                            });
                        });

                        let total_seconds: f64 = match app.interval_mode {
                            IntervalMode::Constant => app.hours as f64 * 3600.0
                                + app.minutes as f64 * 60.0
                                + app.seconds as f64
                                + app.milliseconds as f64 / 1000.0,
                            IntervalMode::Random => app.interval_mode_random_min as f64,
                        };
                        let cps: u32 = (1.0 / total_seconds) as u32;

                        // if total_seconds == 0.0 {
                        //     ui.monospace("Estimated CPS: >10000");
                        // } else if cps > 0 {
                        //     ui.monospace(format!("Estimated CPS: ~{}", cps));
                        // }

                        if cps >= 2000 {
                            let warning_color = Color32::from_rgb(255, 0, 0);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Image::new(egui::include_image!("./assets/Warning.png"))
                                        .tint(warning_color),
                                );
                                ui.colored_label(
                                    warning_color,
                                    RichText::new("Your system may lag much!").monospace(),
                                );
                            });
                        } else if cps >= 200 {
                            let warning_color = Color32::from_rgb(255, 255, 0);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Image::new(egui::include_image!("./assets/Warning.png"))
                                        .tint(warning_color),
                                );
                                ui.colored_label(
                                    warning_color,
                                    RichText::new("Your system may lag!").monospace(),
                                );
                            });
                        }
                    });

                    ui.add_space(20.0);

                    egui::Frame::popup(&ctx.style()).show(ui, |ui| {
                        egui::Frame::popup(&ctx.style())
                            .fill(Color32::from_rgb(0, 170, 255))
                            .show(ui, |ui| {
                                let mut available = ui.available_rect_before_wrap();
                                available.set_height(0.0);
                                ui.allocate_rect(available, Sense::focusable_noninteractive());
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [20.0, 20.0],
                                        egui::Image::new(egui::include_image!("./assets/Cog.png")),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new("Settings")
                                                .heading()
                                                .text_style(egui::TextStyle::Heading)
                                                .color(Color32::WHITE),
                                        )
                                        .selectable(false),
                                    );
                                });
                            });
                        ui.add_space(10.0);

                        if ui
                            .add_sized(
                                [50.0, 50.0],
                                egui::ImageButton::new(match app.mouse_button {
                                    MouseButton::Left => {
                                        egui::include_image!("./assets/MouseLeft.png")
                                    }
                                    MouseButton::Middle => {
                                        egui::include_image!("./assets/MouseMiddle.png")
                                    }
                                    MouseButton::Right => {
                                        egui::include_image!("./assets/MouseRight.png")
                                    }
                                })
                                .frame(false),
                            )
                            .on_hover_and_drag_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            app.mouse_button = match app.mouse_button {
                                MouseButton::Left => MouseButton::Middle,
                                MouseButton::Middle => MouseButton::Right,
                                MouseButton::Right => MouseButton::Left,
                            }
                        };

                        ui.horizontal(|ui| {
                            ui.label("Click Mode");
                            egui::ComboBox::from_id_source("clickmode")
                                .selected_text(format!("{}", app.click_mode.as_ref()))
                                .show_ui(ui, |ui| {
                                    for click_mode in ClickMode::iter() {
                                        ui.selectable_value(
                                            &mut app.click_mode,
                                            click_mode,
                                            click_mode.as_ref(),
                                        );
                                    }
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Limit Mode");
                            egui::ComboBox::from_id_source("limitmode")
                                .selected_text(format!("{}", app.limit_mode.as_ref()))
                                .show_ui(ui, |ui| {
                                    for limit_mode in LimitMode::iter() {
                                        ui.selectable_value(
                                            &mut app.limit_mode,
                                            limit_mode,
                                            limit_mode.as_ref(),
                                        );
                                    }
                                });

                            match app.limit_mode {
                                LimitMode::Clicks => {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut app.limit_mode_clicks_amount)
                                                .speed(1)
                                                .max_decimals(0),
                                        );
                                        ui.label("Clicks");
                                    });
                                }
                                LimitMode::Time => {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut app.limit_mode_time)
                                                .speed(0.25)
                                                .max_decimals(3),
                                        );
                                        ui.label("Seconds");
                                    });
                                }
                                _ => {}
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Color Mode").on_hover_text("If enabled, the auto clicker will only click if the cursor's current\nhovering pixel has the same color as the set Color property.");
                            ui.checkbox(&mut app.color_mode, "");
                            ui.colored_label(Color32::from_rgb(255, 170, 0), "(EXPERIMENTAL)");
                            ui.colored_label(Color32::from_rgb(200, 0, 0), "(VERY SLOW)").on_hover_text("Color Mode is very slow and will cause your auto clicker to click slower.\nIt is intended to be used in situations where the delay doesn't really\nmatter.");
                        });
                        ui.indent("colormode", |ui| {
                            ui.add_enabled_ui(app.color_mode, |ui| {
                                if app.color_mode {
                                    let mouse_location = autopilot::mouse::location();
                                    let result = autopilot::screen::get_color(mouse_location);
                                    if result.is_ok() {
                                        let pixel = result.unwrap();
                                        app.hovering_pixel_color =
                                            Color32::from_rgb(pixel.0[0], pixel.0[1], pixel.0[2]);
                                    }
                                }

                                ui.horizontal(|ui| {
                                    ui.color_edit_button_srgba(&mut app.color_mode_color);
                                    ui.label("Color").on_hover_text("The color of pixel that you need the cursor to hover over for the\nauto clicker to click.");
                                });
                                ui.horizontal(|ui| {
                                    ui.add(egui::Slider::new(
                                        &mut app.color_mode_distance_threshold,
                                        0f32..=1f32,
                                    ));
                                    ui.label("Distance Threshold").on_hover_text("This setting lets you set a threshold distance for the Color property.\n\n0.0 = Color has to be the exact same\n1.0 = Color can be any color (any distance is accepted)");
                                });
                                // ui.horizontal(|ui| {
                                //     ui.color_edit_button_srgba(
                                //         &mut app.hovering_pixel_color.clone(),
                                //     );
                                //     ui.label("Hovering Color");
                                // });
                                // let percentage = percentage_distance_between_colors(
                                //     app.hovering_pixel_color,
                                //     app.color_mode_color,
                                // );
                                // if percentage <= app.color_mode_distance_threshold {
                                //     ui.label("YES");
                                // }
                            });
                        });
                    });

                    ui.add_space(20.0); // Add space equivalent to BottomBar

                    drop(app);
                    let app = self.app();
                    let enabled_changed = app.clicker_enabled != app.last_clicker_enabled;
                    drop(app);

                    if enabled_changed {
                        let mut app = self.app_mut();
                        app.last_clicker_enabled = app.clicker_enabled;

                        if app.clicker_enabled {
                            app.clicker_start_time = Instant::now();
                            drop(app);
                            self.start_clicker();
                        } else {
                            app.try_release_mouse();
                        }
                    }
                });
            });

            // Hide content with transparent black overlay when clicker is enabled
            if self.app().clicker_enabled {
                ui.painter().rect_filled(
                    ui.clip_rect(),
                    Rounding::ZERO,
                    Color32::from_black_alpha(200),
                );
                // ui.painter().text(
                //     ui.clip_rect().center(),
                //     Align2::CENTER_CENTER,
                //     "CLICKING",
                //     FontId::proportional(20.0),
                //     ui.style().visuals.text_color(),
                // );
                egui::Image::new(egui::include_image!("./assets/Click.png")).paint_at(
                    ui,
                    Rect::from_center_size(ui.clip_rect().center(), [50.0, 50.0].into()),
                );
            }
        });

        egui::TopBottomPanel::bottom("BottomBar").show(&ctx, |ui| {
            let start_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F6);
            if ui.input_mut(|i| i.consume_shortcut(&start_shortcut)) {
                self.toggle_clicker();
            }

            egui::menu::bar(ui, |ui| {
                if ui
                    .add(egui::ImageButton::new(if self.app().clicker_enabled {
                        egui::include_image!("./assets/Pause.png")
                    } else {
                        egui::include_image!("./assets/Play.png")
                    }))
                    .clicked()
                {
                    self.toggle_clicker();
                };

                let app = self.app();
                if app.clicker_enabled {
                    ui.label(format!(
                        "Time: {:.2}",
                        Instant::now()
                            .duration_since(app.clicker_start_time)
                            .as_secs_f64()
                    ));
                    ui.label(format!("Clicks: {}", app.total_clicks));
                    if app.click_mode == ClickMode::Toggle {
                        ui.label(format!("Pressed: {}", app.mouse_is_pressed));
                    }
                }

                ui.painter().text(
                    ui.available_rect_before_wrap().right_bottom(),
                    Align2::RIGHT_BOTTOM,
                    env!("CARGO_PKG_VERSION"),
                    egui::FontId::proportional(10.0),
                    ui.style().visuals.text_color(),
                );
            });
        });

        ctx.request_repaint();
    }
}
