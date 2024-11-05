#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example
use eframe::egui;
use egui_plot::{Bar, BoxElem, PlotPoint, PlotPoints};
use std::cmp;
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
//lib
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use stocki::{
    plot::plot,
    types::{MeasurementWindow, StockType},
    utils::{get_data, get_data2},
};

fn main() -> eframe::Result {
    // let args = Args::parse();

    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([980.0, 900.0]),
        ..Default::default()
    };
    let mut app = Stocki::default(1000);

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(app))
        }),
    )
}
pub type Measurement = egui_plot::PlotPoint;

struct Stocki {
    include_y: Vec<f64>,
    measurements: Arc<Mutex<MeasurementWindow>>,
    last_update: Instant, // Add this field to track the last update time
    current_value: f64,
    target_value: f64,
    selected_stock: Arc<Mutex<String>>, // 선택된 주식
    stocks: Vec<String>,                // 사용 가능한 주식 목록
    chart_type: ChartType,
    time_frame: TimeFrame,
}
impl TimeFrame {
    fn to_api_string(&self) -> String {
        match self {
            TimeFrame::Day => "day".to_string(),
            TimeFrame::Week => "week".to_string(),
            TimeFrame::Month => "month".to_string(),
            TimeFrame::Year => "year".to_string(),
        }
    }
}
impl Stocki {
    fn default(look_behind: usize) -> Self {
        // let (tx, rx) = mpsc::channel();
        let selected_type = Arc::new(Mutex::new("day".to_string())); // 초기 주식 이름

        let selected_stock = Arc::new(Mutex::new("AAPL".to_string())); // 초기 주식 이름

        let selected_stock_clone = Arc::clone(&selected_stock);
        let selected_type_clone = Arc::clone(&selected_type);

        let stock_name = selected_stock_clone.lock().unwrap().clone(); // 선택된 주식 이름 가져오기
        let stock_type = selected_type_clone.lock().unwrap().clone(); // 선택된 주식 이름 가져오기
        let new_data = get_data2(&stock_name, &stock_type); // 주식 데이터를 가져옴
        let stocks = vec![
            "AAPL".to_string(),
            "GOOGL".to_string(),
            "MSFT".to_string(),
            "AMZN".to_string(),
            "META".to_string(),
            "TSLA".to_string(),
            "NVDA".to_string(),
        ];
        Self {
            measurements: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(
                look_behind,
                new_data,
            ))),
            include_y: Vec::new(),
            last_update: Instant::now(),
            current_value: 0.0,
            target_value: rand::random::<f64>() * 2.0 - 1.0,
            selected_stock,
            stocks,
            chart_type: ChartType::Line, // 기본값 설정
            time_frame: TimeFrame::Day,  // 기본값 설정
        }
    }
    fn update_target(&mut self) {
        self.target_value = rand::random::<f64>() * 2.0 - 1.0; // New random target between -1 and 1
    }

    fn interpolate_value(&mut self) {
        // Smoothly interpolate current_value towards target_value
        let difference = self.target_value - self.current_value;
        self.current_value += difference * 0.1; // Adjust this factor to control movement speed
    }
    fn update_stock_data(&mut self, stock_name: &str) {
        let stock_type = "day".to_string();
        let new_data = get_data2(stock_name, &stock_type);

        // Lock measurements and update its content
        if let Ok(mut measurements) = self.measurements.lock() {
            *measurements = MeasurementWindow::new_with_look_behind(1000, new_data);
        }
    }
}
////
impl eframe::App for Stocki {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now: Instant = Instant::now();
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::menu::bar(ui, |ui| {
                    let is_web = cfg!(target_arch = "wasm32");
                    if !is_web {
                        ui.menu_button("File", |ui| {
                            if ui.button("Quit").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                        ui.menu_button("View", |ui| {
                            if ui.button("Reset Layout").clicked() {
                                // Reset chart zoom/position
                            }
                        });
                    }
                    ui.add_space(16.0);
                    egui::widgets::global_dark_light_mode_buttons(ui);
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 현재 시간 표시
                    ui.label(format!("Last Updated: {}", now.elapsed().as_secs()));
                });
            });
        });
        egui::SidePanel::left("stock_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Stocks");

                // Stock selector
                ui.group(|ui| {
                    let current_stock = self.selected_stock.lock().unwrap().clone();
                    let mut selected = None;

                    ui.menu_button(&current_stock, |ui| {
                        for stock in &self.stocks {
                            if ui.button(stock).clicked() {
                                selected = Some(stock.clone());
                                ui.close_menu();
                            }
                        }
                    });

                    if let Some(selected_stock) = selected {
                        *self.selected_stock.lock().unwrap() = selected_stock.clone();
                        self.update_stock_data(&selected_stock);
                    }
                });

                ui.group(|ui| {
                    ui.heading("Stock Info");
                    if let Ok(measurements) = self.measurements.lock() {
                        if let PlotPoints::Owned(points) = measurements.plot_values() {
                            if let (Some(last_value), Some(first_value)) = (points.last(), points.first()) {
                                let current_price = last_value.y;
                                let open_price = first_value.y;
    
                                // Calculate price changes
                                let price_change = current_price - open_price;
                                let price_change_percent = (price_change / open_price) * 100.0;
    
                                // Set color based on price change
                                let change_color = if price_change >= 0.0 {
                                    egui::Color32::from_rgb(0, 255, 0) // Green for increase
                                } else {
                                    egui::Color32::from_rgb(255, 0, 0) // Red for decrease
                                };
    
                                // Display current price
                                ui.label(format!("Current Price: ${:.2}", current_price));
    
                                // Display price change
                                ui.colored_label(
                                    change_color,
                                    format!(
                                        "Change: {}{:.2}$ ({:.2}%)",
                                        if price_change >= 0.0 { "+" } else { "" },
                                        price_change,
                                        price_change_percent
                                    ),
                                );
                            if let Some(volume) = measurements.volumes().last() {
                                let formatted_volume = if *volume >= 1_000_000.0 {
                                    format!("{:.1}M", volume / 1_000_000.0)
                                } else if *volume >= 1_000.0 {
                                    format!("{:.1}K", volume / 1_000.0)
                                } else {
                                    format!("{:.0}", volume)
                                };
                                ui.label(format!("Volume: {}", formatted_volume));
                            }
                            // 고가/저가 계산
                            let high_price =
                                points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                            let low_price =
                                points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);

                            ui.label(format!("High: ${:.2}", high_price));
                            ui.label(format!("Low: ${:.2}", low_price));

                            // 시간 정보
                            ui.label(format!(
                                "Time Frame: {}",
                                match self.time_frame {
                                    TimeFrame::Day => "1 Day",
                                    TimeFrame::Week => "1 Week",
                                    TimeFrame::Month => "1 Month",
                                    TimeFrame::Year => "1 Year",
                                }
                            ));
                        }
                    }
                }
            });
        });


        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Chart Controls
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.chart_type, ChartType::Candle, "Candle");
                    ui.selectable_value(&mut self.chart_type, ChartType::Line, "Line");
                    ui.add_space(32.0);
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Day, "1D");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Week, "1W");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Month, "1M");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Year, "1Y");
                });

                // Main Chart
                ui.group(|ui| {
                    let mut plot = egui_plot::Plot::new("stock_chart")
                        .height(400.0)
                        .include_x(0.0)
                        .include_x(100.0)
                        .label_formatter(|name, value| format!("{}: ${:.2}", name, value.y));

                    for y in self.include_y.iter() {
                        plot = plot.include_y(*y);
                    }

                    plot.show(ui, |plot_ui| {
                        if let Ok(measurements) = self.measurements.lock() {
                            match self.chart_type {
                                ChartType::Line => {
                                    plot_ui.line(
                                        egui_plot::Line::new(measurements.plot_values())
                                            .color(egui::Color32::from_rgb(0, 150, 255))
                                            .width(2.0),
                                    );
                                }
                                ChartType::Candle => {
                                    // Implement candle chart rendering
                                }
                            }
                        }
                    });
                });

                // Volume Chart
                ui.group(|ui| {
                    let plot = egui_plot::Plot::new("volume_chart")
                        .height(100.0)
                        .include_x(0.0)
                        .include_x(100.0);

                    plot.show(ui, |plot_ui| {
                        // Add volume bars here
                    });
                });
            });
        });
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Order Book");
                        // Add order book implementation
                    });
                    ui.vertical(|ui| {
                        ui.heading("Recent Trades");
                        // Add recent trades implementation
                    });
                });
            });
        ctx.request_repaint();
    }
}
// Add these enums to your Stocki struct
#[derive(PartialEq, Clone, Debug)] // derive 속성 추가
enum ChartType {
    Candle,
    Line,
}
#[derive(PartialEq, Clone, Debug)] // derive 속성 추가
enum TimeFrame {
    Day,
    Week,
    Month,
    Year,
}
