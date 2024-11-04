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

use stocki::{plot::plot, types::StockType, utils::get_data};

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
impl MeasurementWindow {
    pub fn new_with_look_behind(look_behind: usize) -> Self {
        Self {
            values: BTreeMap::new(),
            look_behind,
            start_time: Instant::now(),
        }
    }

    pub fn add(&mut self, x: f64, y: f64) {
        // 현재 시간
        let now = Instant::now();

        // 기준 시간 계산
        let limit_time = now - Duration::from_secs(self.look_behind as u64);

        // 오래된 값 제거
        self.values.retain(|&key, _| {
            let timestamp = Instant::now() - Duration::from_secs(key);
            timestamp >= limit_time
        });

        // 측정값 추가 (동일한 x 값이 있을 경우 대체됨)
        self.values.insert(x as u64, y);
    }

    pub fn plot_values(&self) -> PlotPoints {
        // BTreeMap에서 y 값을 추출하여 Vec<(f64, f64)> 형태로 변환
        let points: Vec<PlotPoint> = self
            .values
            .iter()
            .map(|(key, &value)| PlotPoint {
                x: *key as f64,
                y: value,
            }) // Float64를 f64로 변환
            .collect();

        // PlotPoints로 변환하여 반환
        egui_plot::PlotPoints::Owned(points)
    }
}

struct Stocki {
    // button_text: Arc<Mutex<String>>, // 선택한 주식 이름을 공유하는 Arc<Mutex>
    // stocks: Vec<&'static str>,       // 주식 이름을 저장할 벡터
    // stock_types: Vec<&'static str>,  // 주식 이름을 저장할 벡터

    // stock_data: Vec<BoxElem>,   // 주식 데이터
    // last_update: Instant,       // 마지막 업데이트 시간
    // rx: Receiver<Vec<BoxElem>>, // 수신자
    // stock_type: Arc<Mutex<String>>,
    include_y: Vec<f64>,
    measurements: Arc<Mutex<MeasurementWindow>>,
    last_update: Instant, // Add this field to track the last update time
    current_value: f64,
    target_value: f64,
}
#[derive(Debug)]
pub struct MeasurementWindow {
    pub values: BTreeMap<u64, f64>,
    pub look_behind: usize,
    start_time: Instant,
}
impl Stocki {
    fn default(look_behind: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let selected_type = Arc::new(Mutex::new("day".to_string())); // 초기 주식 이름

        let selected_stock = Arc::new(Mutex::new("AAPL".to_string())); // 초기 주식 이름

        let selected_stock_clone = Arc::clone(&selected_stock);
        let selected_type_clone = Arc::clone(&selected_type);

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1)); // 30초 대기
                let stock_name = selected_stock_clone.lock().unwrap().clone(); // 선택된 주식 이름 가져오기
                let stock_type = selected_type_clone.lock().unwrap().clone(); // 선택된 주식 이름 가져오기

                let new_data = get_data(&stock_name, &stock_type); // 주식 데이터를 가져옴
                if tx.send(new_data).is_err() {
                    break; // 메인 스레드가 더 이상 데이터를 수신하지 않으면 종료
                }
            }
        });
        Self {
            measurements: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(
                look_behind,
            ))),
            include_y: Vec::new(),
            last_update: Instant::now(), // Initialize the last update time
            current_value: 0.0,
            target_value: rand::random::<f64>() * 2.0 - 1.0, // Random value between -1 and 1
        }
        // Self {
        //     button_text: selected_stock,
        //     stocks: vec!["AAPL", "GOOGL"],
        //     stock_types: vec!["Day", "YEAR"],
        //     stock_data: get_data("AAPL", "Day"), // 초기 주식 데이터
        //     last_update: Instant::now(),
        //     rx,
        //     stock_type: selected_type, // 여기에 필드 이름을 추가합니다.
        //     include_y: Vec::new(),
        // }
    }
    fn update_target(&mut self) {
        self.target_value = rand::random::<f64>() * 2.0 - 1.0; // New random target between -1 and 1
    }

    fn interpolate_value(&mut self) {
        // Smoothly interpolate current_value towards target_value
        let difference = self.target_value - self.current_value;
        self.current_value += difference * 0.1; // Adjust this factor to control movement speed
    }
}
////
impl eframe::App for Stocki {
    // fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    //     // while let Ok(new_data) = self.rx.try_recv() {
    //     //     println!("새 데이터 수신");
    //     //     self.stock_data = new_data;
    //     //     self.last_update = Instant::now(); // 마지막 업데이트 시간 갱신

    //     //     ctx.request_repaint();
    //     // }
    //     // ctx.request_repaint_after(Duration::from_secs(1));
    //     if let Ok(new_data) = self.rx.try_recv() {
    //         self.stock_data = new_data;
    //         self.last_update = Instant::now();
    //         ctx.request_repaint(); // 데이터 갱신 시에만 UI를 갱신
    //     } else {
    //         // 특정 시간 간격마다 자동 UI 리페인트
    //         ctx.request_repaint_after(Duration::from_secs(1));
    //     }
    //     // 상단 바
    //     egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
    //         egui::menu::bar(ui, |ui| {
    //             let is_web = cfg!(target_arch = "wasm32");
    //             if !is_web {
    //                 ui.menu_button("File", |ui| {
    //                     if ui.button("Quit").clicked() {
    //                         ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    //                     }
    //                 });
    //                 ui.add_space(16.0);
    //             }
    //             egui::widgets::global_dark_light_mode_buttons(ui);
    //         });
    //     });

    //     egui::CentralPanel::default().show(ctx, |ui| {
    //         ui.horizontal(|ui| {
    //             ui.vertical(|ui| {
    //                 ui.group(|ui| {
    //                     let button_text = self.button_text.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
    //                     let stock_type = self.stock_type.lock().unwrap().clone(); // 공유된 주식 이름 가져오기

    //                     ui.menu_button(&button_text, |ui| {
    //                         for &stock in &self.stocks {
    //                             if ui.button(stock).clicked() {
    //                                 println!("{} 클릭", stock);
    //                                 *self.button_text.lock().unwrap() = stock.to_string(); // 선택된 주식 이름 업데이트
    //                                 self.stock_data = get_data(stock,&stock_type); // 클릭 시 즉시 데이터 업데이트
    //                                 ui.close_menu(); // 메뉴 닫기
    //                             }
    //                         }
    //                     });
    //                 });
    //                 ui.group(|ui| {
    //                     ui.label("Candle Chart Section");
    //                     // plot::bar_chart(ui, &self.stock_data);
    //                 });
    //                 ui.group(|ui| {
    //                     ui.label("Candle Chart Section");
    //                     let button_text = self.button_text.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
    //                     let stock_type = self.stock_type.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
    //                     ui.horizontal(|ui| {

    //                         ui.menu_button(&stock_type, |ui| {
    //                             for &stock in &self.stock_types {
    //                                 if ui.button(stock).clicked() {
    //                                     println!("{} 클릭", stock);
    //                                     *self.stock_type.lock().unwrap() = stock.to_string(); // 선택된 주식 이름 업데이트
    //                                     self.stock_data = get_data(&button_text,&stock_type); // 클릭 시 즉시 데이터 업데이트
    //                                     ui.close_menu(); // 메뉴 닫기
    //                                 }
    //                             }
    //                         });

    //                     });

    //                     plot::bar_chart(ui, &self.stock_data);
    //                 });
    //             });
    //         });
    //     });
    // }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now: Instant = Instant::now();

        // Update target value every 3 seconds
        if self.last_update.elapsed() >= Duration::from_secs(3) {
            self.update_target();
            self.last_update = now;
        }
        let elapsed = now
            .duration_since(self.measurements.lock().unwrap().start_time)
            .as_secs_f64();
        self.current_value += 1.;

        self.measurements
            .lock()
            .unwrap()
            .add(elapsed, self.current_value);
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.group(|ui| {
                       // let button_text = self.button_text.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
                        // let stock_type = self.stock_type.lock().unwrap().clone(); // 공유된 주식 이름 가져오기

                        // ui.menu_button(&button_text, |ui| {
                        //     for &stock in &self.stocks {
                        //         if ui.button(stock).clicked() {
                        //             println!("{} 클릭", stock);
                        //             *self.button_text.lock().unwrap() = stock.to_string(); // 선택된 주식 이름 업데이트
                        //             self.stock_data = get_data(stock, &stock_type); // 클릭 시 즉시 데이터 업데이트
                        //             ui.close_menu(); // 메뉴 닫기
                        //         } 
                        //     }
                        // });
                    });
                    ui.group(|ui| {
                        ui.label("Candle Chart Section");
                        // plot::bar_chart(ui, &self.stock_data);
                    });
                    ui.group(|ui| {
                        let mut plot = egui_plot::Plot::new("measurements");
                        for y in self.include_y.iter() {
                            plot = plot.include_y(*y);
                        }

                        plot.show(ui, |plot_ui| {
                            plot_ui.line(egui_plot::Line::new(
                                self.measurements.lock().unwrap().plot_values(),
                            ));
                        });
                        // ui.label("Candle Chart Section");
                        // let button_text = self.button_text.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
                        // let stock_type = self.stock_type.lock().unwrap().clone(); // 공유된 주식 이름 가져오기
                        // ui.horizontal(|ui| {
                        //     ui.menu_button(&stock_type, |ui| {
                        //         for &stock in &self.stock_types {
                        //             if ui.button(stock).clicked() {
                        //                 println!("{} 클릭", stock);
                        //                 *self.stock_type.lock().unwrap() = stock.to_string(); // 선택된 주식 이름 업데이트
                        //                 self.stock_data = get_data(&button_text, &stock_type); // 클릭 시 즉시 데이터 업데이트
                        //                 ui.close_menu(); // 메뉴 닫기
                        //             }
                        //         }
                        //     });
                        // });

                        // plot::bar_chart(ui, &self.stock_data);
                    });
                });
            });
        });
        ctx.request_repaint();
    }
}
