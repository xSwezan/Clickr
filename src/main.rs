#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, sleep},
    time::{Duration, Instant},
};

use eframe::{
    egui::{
        self, Align2, Color32, FontDefinitions, FontFamily, IconData, Image, KeyboardShortcut,
        Layout, Margin, Rect, Response, RichText, Rounding, Sense, Vec2,
    },
    CreationContext,
};
use egui_extras::{Column, TableBuilder};
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
    Single,
    Double,
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

const COMPACT_WINDOW_SIZE: Vec2 = Vec2::new(240.0, 80.0);
const WINDOW_SIZE: Vec2 = Vec2::new(400.0, 410.0);
const TOGGLE_AUTO_CLICKER_SHORTCUT: KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F6);

fn percentage_distance_between_colors(a: Color32, b: Color32) -> f32 {
    let distance_r = a.r().abs_diff(b.r()) as f32;
    let distance_g = a.g().abs_diff(b.g()) as f32;
    let distance_b = a.b().abs_diff(b.b()) as f32;

    let distance = (distance_r.powi(2) + distance_g.powi(2) + distance_b.powi(2)).sqrt();
    let percentage = distance / 441.672956;

    percentage
}

fn tag_label(ui: &mut egui::Ui, text: &str, color: Color32, icon: Option<Image>) {
    egui::Frame::default()
        .fill(color)
        .inner_margin(Margin::symmetric(5.0, 0.0))
        .outer_margin(Margin::ZERO)
        .rounding(Rounding::same(3.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                if icon.is_some() {
                    ui.add_sized([12.0, 12.0], icon.unwrap());
                }

                ui.label(
                    egui::RichText::new(text)
                        .color(egui::Color32::WHITE)
                        .size(10.0),
                );
            });
        });
}

fn beta_tag(ui: &mut egui::Ui) {
    tag_label(ui, "BETA", Color32::from_rgb(0, 170, 255), None);
}

fn danger_tag(ui: &mut egui::Ui, text: &str) {
    tag_label(
        ui,
        text,
        Color32::from_rgb(255, 0, 0),
        Some(Image::new(egui::include_image!("./assets/Warning.png"))),
    );
}

fn warning_tag(ui: &mut egui::Ui, text: &str) {
    tag_label(
        ui,
        text,
        Color32::from_rgb(230, 140, 0),
        Some(Image::new(egui::include_image!("./assets/Warning.png"))),
    );
}

fn setting_label(ui: &mut egui::Ui, text: &str) -> Response {
    ui.label(
        RichText::new(text).color(ui.style().visuals.text_color()),
        // .family(egui::FontFamily::Name("InterBold".into()))
    )
}

fn big_header(ui: &mut egui::Ui, text: &str, image: Image) {
    egui::Frame::popup(&ui.ctx().style())
        .fill(Color32::from_rgb(0, 170, 255))
        .show(ui, |ui| {
            let mut available = ui.available_rect_before_wrap();
            available.set_height(0.0);
            ui.allocate_rect(available, Sense::focusable_noninteractive());
            ui.horizontal(|ui| {
                ui.add_sized([20.0, 20.0], image);
                ui.add(
                    egui::Label::new(
                        RichText::new(text)
                            .heading()
                            .text_style(egui::TextStyle::Heading)
                            .color(Color32::WHITE),
                    )
                    .selectable(false),
                );
            });
        });
    ui.add_space(10.0);
}

fn show_constant_interval_mode(ui: &mut egui::Ui, h: &mut u32, m: &mut u32, s: &mut u32, ms: &mut u32) {
	ui.columns(4, |columns| {
		columns[0].add(egui::DragValue::new(h).range(0..=23).suffix("h"));
		columns[1].add(egui::DragValue::new(m).range(0..=59).suffix("m"));
		columns[2].add(egui::DragValue::new(s).range(0..=59).suffix("s"));
		columns[3].add(egui::DragValue::new(ms).range(0..=999).suffix("ms"));
	});
}

fn show_random_interval_mode(ui: &mut egui::Ui, min: &mut f32, max: &mut f32) {
	ui.columns(2, |columns| {
		// Clamp max between 0.0 and 3600.0
		if *max > 3600.0 {
			*max = 3600.0;
		} else if *max < 0.0 {
			*max = 0.0;
		}

		// Clamp min between 0.0 and max
		if *min > *max {
			*min = *max;
		} else if *min < 0.0 {
			*min = 0.0;
		}

		let fields = [min, max];
		fields.into_iter().enumerate().for_each(|(i, value)| {
			columns[i].add(egui::DragValue::new(value)
				.suffix("s")
				.speed(0.1)
				.min_decimals(1)
				.range(0..=3600)
				.update_while_editing(false)
				.max_decimals(3),
			);
		});
	});
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
                .with_inner_size([400.0, 410.0])
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
            Ok(Box::new(AppHolder::new(cc)))
        }),
    )
}

struct AppHolder {
    main_app: Arc<Mutex<App>>,
}

impl AppHolder {
    fn new(cc: &CreationContext<'_>) -> Self {
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
            click_mode: ClickMode::Single,

            mouse_is_pressed: false,

            clicker_id: 0,

            color_mode: false,
            color_mode_color: Color32::BLACK,
            hovering_pixel_color: Color32::BLACK,

            limit_mode: LimitMode::None,
            limit_mode_clicks_amount: 10,
            color_mode_distance_threshold: 0,
            limit_mode_time: 1.0,

            clicker_enabled: false,
            last_clicker_enabled: false,
            clicker_start_time: Instant::now(),
            total_clicks: 0,

            always_on_top: true,
            focused: true,
            compact_mode: false,
        };

        let app_arc = Arc::new(Mutex::new(new_app));
        let app_arc_clone = app_arc.clone();

        KeybdKey::F6Key.bind(move || {
            let mut app = app_arc_clone.lock().unwrap();
            app.clicker_enabled = !app.clicker_enabled;
        });

        thread::spawn(|| inputbot::handle_input_events());

        let mut fonts = FontDefinitions::default();

        // Inter Bold
        fonts.font_data.insert(
            "InterBold".to_owned(),
            egui::FontData::from_static(include_bytes!("./assets/fonts/InterBold.ttf")),
        );
        fonts.families.insert(
            FontFamily::Name("InterBold".into()),
            vec!["InterBold".to_owned()],
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "InterBold".to_owned());

        // Inter Regular
        fonts.font_data.insert(
            "InterRegular".to_owned(),
            egui::FontData::from_static(include_bytes!("./assets/fonts/InterRegular.ttf")),
        );
        fonts.families.insert(
            FontFamily::Name("InterRegular".into()),
            vec!["InterRegular".to_owned()],
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "InterRegular".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        AppHolder { main_app: app_arc }
    }

    fn app(&self) -> MutexGuard<App> {
        self.main_app.lock().unwrap()
    }
    fn app_mut(&mut self) -> MutexGuard<App> {
        self.main_app.lock().unwrap()
    }

    fn click_shield(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(Margin::same(10.0)))
            .show(ctx, |ui| {
                // Hide content with transparent black overlay when clicker is enabled
                ui.painter().rect_filled(
                    ui.clip_rect(),
                    Rounding::ZERO,
                    Color32::from_black_alpha(200),
                );
                egui::Image::new(egui::include_image!("./assets/Click.png")).paint_at(
                    ui,
                    Rect::from_center_size(ui.clip_rect().center(), [50.0, 50.0].into()),
                );
				egui::Grid::new("click_shield_grid").show(ui, |ui| {
					let app = self.app();

					ui.label("Time");
					ui.label(
						RichText::new(format!(
							"{:.2}",
							Instant::now()
								.duration_since(app.clicker_start_time)
								.as_secs_f64()
						))
						.color(ui.style().visuals.strong_text_color()),
					);
					ui.end_row();

					ui.label("Clicks");
					ui.label(
						RichText::new(format!("{}", app.total_clicks))
							.color(ui.style().visuals.strong_text_color()),
					);
					ui.end_row();
				});

				if self.app().focused {
					ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
						warning_tag(ui, "UNFOCUS THE WINDOW TO CLICK!");
					});
				}
			});
    }

    fn compact_click_shield(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(Margin::same(10.0)))
            .show(ctx, |ui| {
                // Hide content with transparent black overlay when clicker is enabled
                ui.painter().rect_filled(
                    ui.clip_rect(),
                    Rounding::ZERO,
                    Color32::from_black_alpha(200),
                );
                egui::Image::new(egui::include_image!("./assets/Click.png")).paint_at(
                    ui,
                    Rect::from_min_size(ui.clip_rect().right_center(), [50.0, 50.0].into()),
                );

                egui::Grid::new("compact_click_shield_grid").show(ui, |ui| {
                    let app = self.app();

                    ui.label("Time");
                    ui.label(
                        RichText::new(format!(
                            "{:.2}",
                            Instant::now()
                                .duration_since(app.clicker_start_time)
                                .as_secs_f64()
                        ))
                        .color(ui.style().visuals.strong_text_color()),
                    );
                    ui.end_row();

                    ui.label("Clicks");
                    ui.label(
                        RichText::new(format!("{}", app.total_clicks))
                            .color(ui.style().visuals.strong_text_color()),
                    );
                    ui.end_row();
                });
            });
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            if ui.input_mut(|i| i.consume_shortcut(&TOGGLE_AUTO_CLICKER_SHORTCUT)) {
                self.toggle_clicker();
            }

            ctx.input(|i| {
                self.app_mut().focused = i.viewport().focused.unwrap();
            });

            egui::menu::bar(ui, |ui| {
				ui.menu_button("Actions", |ui| {
                    if ui
                        .add(
                            egui::Button::new(if self.app().clicker_enabled {
                                "Stop Auto Clicker"
                            } else {
                                "Start Auto Clicker"
                            })
                            .shortcut_text(ui.ctx().format_shortcut(&TOGGLE_AUTO_CLICKER_SHORTCUT)),
                        )
                        .clicked()
                    {
                        self.toggle_clicker();
                    }

                    if ui
                        .checkbox(&mut self.app_mut().compact_mode, "Compact Mode")
                        .clicked()
                    {
                        if self.app().compact_mode {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                COMPACT_WINDOW_SIZE,
                            ));
                        } else {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(WINDOW_SIZE));
                        }
                    }
                });

				if !self.app().compact_mode {
					ui.separator();

					if ui.selectable_label(true, "Home").clicked() {

					}

					if ui.selectable_label(false, "Settings").clicked() {

					}

					if ui.selectable_label(false, "Keybinds").clicked() {

					}
				}

                ui.painter().text(
                    ui.available_rect_before_wrap().right_center(),
                    Align2::RIGHT_CENTER,
                    env!("CARGO_PKG_VERSION"),
                    egui::FontId::proportional(10.0),
                    ui.style().visuals.weak_text_color(),
                );
            });
        });
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

            let should_click: bool = !app.focused
                && (!app.color_mode
                    || (app.color_mode
                        && percentage_distance_between_colors(
                            app.hovering_pixel_color,
                            app.color_mode_color,
                        ) <= app.color_mode_distance_threshold as f32 / 255.0));

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

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
			let enabled = !self.app().clicker_enabled;
			ui.add_enabled_ui(enabled, |ui| {
				let mut app = self.app_mut();

				egui::Frame::popup(&ui.ctx().style()).show(ui, |ui| {
					big_header(ui, "Click Interval", egui::Image::new(egui::include_image!("./assets/ClickInterval.png")));

					ui.vertical(|ui| {
						if ui
							.radio(app.interval_mode == IntervalMode::Constant, "Constant")
							.clicked()
						{
							app.interval_mode = IntervalMode::Constant;
						}

						ui.add_enabled_ui(app.interval_mode == IntervalMode::Constant, |ui| {
							let mut h = app.hours;
							let mut m = app.minutes;
							let mut s = app.seconds;
							let mut ms = app.milliseconds;

							show_constant_interval_mode(ui, &mut h, &mut m, &mut s, &mut ms);

							app.hours = h;
							app.minutes = m;
							app.seconds = s;
							app.milliseconds = ms;
						});

						ui.add_space(15.0);

						if ui
							.radio(
								app.interval_mode == IntervalMode::Random,
								"Random Interval",
							)
							.clicked()
						{
							app.interval_mode = IntervalMode::Random;
						}

						ui.add_enabled_ui(app.interval_mode == IntervalMode::Random, |ui| {
							let mut min = app.interval_mode_random_min;
							let mut max = app.interval_mode_random_max;

							show_random_interval_mode(ui, &mut min, &mut max);

							app.interval_mode_random_min = min;
							app.interval_mode_random_max = max;
						});
						ui.add_enabled_ui(app.interval_mode == IntervalMode::Random, |ui| {
							ui.columns(2, |columns| {
								let fields = ["Min", "Max"];
								fields.into_iter().enumerate().for_each(|(i, text)| {
									columns[i].add(egui::Label::new(text));
								});
							});
						});
					});

					let total_seconds: f64 = match app.interval_mode {
						IntervalMode::Constant => {
							app.hours as f64 * 3600.0
								+ app.minutes as f64 * 60.0
								+ app.seconds as f64
								+ app.milliseconds as f64 / 1000.0
						}
						IntervalMode::Random => app.interval_mode_random_max as f64,
					};
					let cps: u32 = (1.0 / total_seconds) as u32;


					if cps >= 2000 {
						danger_tag(ui, "YOUR SYSTEM MAY SLOW DOWN!");
					} else if cps >= 200 {
						ui.vertical_centered(|ui| {
							warning_tag(ui, "YOUR SYSTEM MAY SLOW DOWN!");
						});
					}
				});

				ui.add_space(15.0);

				egui::Frame::popup(&ui.ctx().style()).show(ui, |ui| {
					big_header(ui, "Settings", egui::Image::new(egui::include_image!("./assets/Cog.png")));

					const ROW_HEIGHT: f32 = 20.0;
					TableBuilder::new(ui)
						.column(Column::auto().resizable(false))
						.column(Column::remainder())
						.striped(true)
						.resizable(false)
						.body(|mut body| {
							body.row(ROW_HEIGHT, |mut row| {
								row.col(|ui| {
									setting_label(ui, "Mouse Button");
								});
								row.col(|ui| {
									egui::ComboBox::from_id_source("mousebutton")
										.selected_text(format!("{}", app.mouse_button.as_ref()))
										.show_ui(ui, |ui| {
											for mouse_button in MouseButton::iter() {
												ui.selectable_value(
													&mut app.mouse_button,
													mouse_button,
													mouse_button.as_ref(),
												);
											}
										});
								});
							});
							body.row(ROW_HEIGHT, |mut row| {
								row.col(|ui| {
									setting_label(ui, "Click Mode");
								});
								row.col(|ui| {
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
							});
							body.row(ROW_HEIGHT, |mut row| {
								row.col(|ui| {
									setting_label(ui, "Limit Mode");
								});
								row.col(|ui| {
									ui.horizontal(|ui| {
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
														egui::DragValue::new(
															&mut app.limit_mode_clicks_amount,
														)
														.speed(1)
														.max_decimals(0),
													);
													ui.label("Clicks");
												});
											}
											LimitMode::Time => {
												ui.horizontal(|ui| {
													ui.add(
														egui::DragValue::new(
															&mut app.limit_mode_time,
														)
														.speed(0.25)
														.max_decimals(3),
													);
													ui.label("Seconds");
												});
											}
											_ => {}
										}
									});
								});
							});
							body.row(ROW_HEIGHT, |mut row| {
								row.col(|ui| {
									ui.horizontal(|ui| {
										setting_label(ui, "Color Mode").on_hover_text("If enabled, the auto clicker will only click if the cursor's current\nhovering pixel has the same color as the set Color property.");
										beta_tag(ui);
									});
								});
								row.col(|ui| {
									ui.horizontal(|ui| {
										ui.checkbox(&mut app.color_mode, "");
										ui.add_space(-10.0);
										if app.color_mode {
											egui::CollapsingHeader::new("Settings").show_unindented(ui, |ui| {
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
													ui.add(egui::DragValue::new(&mut app.color_mode_distance_threshold).range(0u8..=255u8));
													ui.label("Threshold").on_hover_text("This setting lets you set a threshold distance for the Color property.\n\n0.0 = Color has to be the exact same\n1.0 = Color can be any color (any distance is accepted)");
												});
											});
										}
									});
								});
							});
							body.row(ROW_HEIGHT, |mut row| {
								row.col(|ui| {
									ui.horizontal(|ui| {
										setting_label(ui, "Always On Top");
										beta_tag(ui);
									});
								});
								row.col(|ui| {
									if ui.checkbox(&mut app.always_on_top, "").clicked() {
										if app.always_on_top {
											ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop))
										} else {
											ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal))
										}
									}
								});
							});
						});
				});
			});
		});
    }

    fn show_compact_menu(&mut self, ui: &mut egui::Ui) {
		let enabled = !self.app().clicker_enabled;
		ui.add_enabled_ui(enabled, |ui| {
			ui.with_layout(
				Layout::centered_and_justified(egui::Direction::LeftToRight),
				|ui| {
					let mut app = self.app_mut();

					match app.interval_mode {
						IntervalMode::Constant => {
							let mut h = app.hours;
							let mut m = app.minutes;
							let mut s = app.seconds;
							let mut ms = app.milliseconds;

							show_constant_interval_mode(ui, &mut h, &mut m, &mut s, &mut ms);

							app.hours = h;
							app.minutes = m;
							app.seconds = s;
							app.milliseconds = ms;
						}
						IntervalMode::Random => {
							let mut min = app.interval_mode_random_min;
							let mut max = app.interval_mode_random_max;

							show_random_interval_mode(ui, &mut min, &mut max);

							app.interval_mode_random_min = min;
							app.interval_mode_random_max = max;
						}
					}

					// ui.label(
					//     RichText::new("Compact Mode").color(ui.style().visuals.strong_text_color()),
					// );
				},
			);
		});
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
    color_mode_distance_threshold: u8,
    hovering_pixel_color: Color32,

    limit_mode: LimitMode,
    limit_mode_clicks_amount: u32,
    limit_mode_time: f32,

    clicker_enabled: bool,
    last_clicker_enabled: bool,
    clicker_start_time: Instant,
    total_clicks: u32,

    always_on_top: bool,
    focused: bool,
    compact_mode: bool,
}

impl App {
    fn click_mouse(&self) {
        let button = match self.mouse_button {
            MouseButton::Left => mouse_rs::types::keys::Keys::LEFT,
            MouseButton::Middle => mouse_rs::types::keys::Keys::MIDDLE,
            MouseButton::Right => mouse_rs::types::keys::Keys::RIGHT,
        };

        match self.click_mode {
            ClickMode::Single => self.mouse.click(&button).expect("Unable to click button"),
            ClickMode::Double => {
                self.mouse.click(&button).expect("Unable to click button");
                self.mouse.click(&button).expect("Unable to click button");
            }
            ClickMode::Toggle => {
                if self.mouse_is_pressed {
                    self.mouse.press(&button).expect("Unable to press button");
                } else {
                    self.mouse
                        .release(&button)
                        .expect("Unable to release button");
                }
            }
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
        self.menu_bar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.app().compact_mode {
                self.show_compact_menu(ui);
            } else {
                self.show_menu(ui);
            }
        });

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

        if self.app().clicker_enabled {
            if self.app().compact_mode {
                self.compact_click_shield(ctx);
            } else {
                self.click_shield(ctx);
            }
        }

        ctx.request_repaint();
    }
}
