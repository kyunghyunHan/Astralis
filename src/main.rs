#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example
use eframe::egui::{self, Id, RichText, Vec2b};
use egui_plot::{Bar, BoxElem, BoxSpread, PlotPoint, PlotPoints};
use std::collections::BTreeMap;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use stocki::{types::{ChartType, MAPeriod, MeasurementWindow, StockData, TimeFrame},utils::colors};

fn main() -> eframe::Result {
    // let args = Args::parse();

    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([980.0, 900.0]),
        ..Default::default()
    };
    let app: Stocki = Stocki::default(1000);

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
    measurements: Arc<Mutex<MeasurementWindow>>,
    selected_stock: Arc<Mutex<String>>, // 선택된 주식
    stocks: Vec<String>,                // 사용 가능한 주식 목록
    chart_type: ChartType,
    time_frame: TimeFrame,
    // show_ma: bool,
    ma_states: std::collections::HashMap<MAPeriod, bool>, // 각 MA의 활성화 상태만 유지
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
        let new_data = StockData::get_data(&stock_name, &stock_type); // 주식 데이터를 가져옴
        let stocks = vec![
            "AAPL".to_string(),
            "GOOGL".to_string(),
            "MSFT".to_string(),
            "AMZN".to_string(),
            "META".to_string(),
            "TSLA".to_string(),
            "NVDA".to_string(),
        ];
        let mut ma_states = std::collections::HashMap::new();

        ma_states.insert(MAPeriod::MA5, false);
        ma_states.insert(MAPeriod::MA10, false);
        ma_states.insert(MAPeriod::MA20, false);
        ma_states.insert(MAPeriod::MA60, false);
        ma_states.insert(MAPeriod::MA224, false);
        Self {
            measurements: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(
                look_behind,
                new_data,
            ))),
            selected_stock,
            stocks,
            chart_type: ChartType::Line,
            time_frame: TimeFrame::Day,
            // show_ma: false,
            ma_states,
        }
    }
    fn calculate_moving_average(&self, prices: &[PlotPoint], period: usize) -> Vec<PlotPoint> {
        let mut ma_values = Vec::new();

        if period == 0 || period > prices.len() {
            return ma_values;
        }

        for i in (period - 1)..prices.len() {
            let window_start = i + 1 - period;
            let window_end = i + 1;
            let sum: f64 = prices[window_start..window_end].iter().map(|p| p.y).sum();
            let ma = sum / period as f64;
            let x = prices[i].x;
            ma_values.push(PlotPoint::new(x, ma));
        }

        ma_values
    }
    fn calculate_rsi(&self, prices: &BTreeMap<u64, StockData>) -> Vec<(f64, f64)> {
        // (timestamp, rsi_value)
        let period = 14;
        if prices.len() < period + 1 {
            return vec![];
        }

        // timestamps와 가격 데이터를 벡터로 변환
        let price_data: Vec<(u64, &StockData)> = prices.iter().map(|(&k, v)| (k, v)).collect();

        let mut rsi_values = Vec::new();
        let mut gains = Vec::new();
        let mut losses = Vec::new();

        // 첫 번째 변화량 계산
        for i in 1..price_data.len() {
            let price_change = price_data[i].1.close - price_data[i - 1].1.close;
            if price_change >= 0.0 {
                gains.push(price_change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-price_change);
            }
        }

        // 초기 평균 계산
        let mut avg_gain = gains[..period].iter().sum::<f64>() / period as f64;
        let mut avg_loss = losses[..period].iter().sum::<f64>() / period as f64;

        // 첫 RSI 계산 (period + 1 인덱스의 타임스탬프 사용)
        let mut rsi = 100.0 - (100.0 / (1.0 + avg_gain / avg_loss));
        let normalized_timestamp = (price_data[period].0 as f64); // 일 단위 정규화
        rsi_values.push((normalized_timestamp, rsi));

        // 나머지 기간에 대한 RSI 계산
        for i in period..gains.len() {
            avg_gain = (avg_gain * (period - 1) as f64 + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period - 1) as f64 + losses[i]) / period as f64;

            if avg_loss == 0.0 {
                rsi = 100.0;
            } else {
                rsi = 100.0 - (100.0 / (1.0 + avg_gain / avg_loss));
            }

            // 해당 시점의 타임스탬프를 정규화하여 저장
            let normalized_timestamp = (price_data[i + 1].0 as f64); // 일 단위 정규화
            rsi_values.push((normalized_timestamp, rsi));
        }

        rsi_values
    }
    // 매매 신호 생성 함수도 PlotPoint를 사용하도록 수정
    fn generate_signals(&self, short_ma: &[PlotPoint], long_ma: &[PlotPoint]) -> Vec<PlotPoint> {
        let mut signals = Vec::new();

        // 주의: x 값을 그대로 인덱스로 사용하지 않고, 실제 데이터 포인트의 x 값을 사용
        for i in 1..short_ma.len().min(long_ma.len()) {
            let prev_short = short_ma[i - 1].y;
            let prev_long = long_ma[i - 1].y;
            let curr_short = short_ma[i].y;
            let curr_long = long_ma[i].y;

            // 골든 크로스 (단기선이 장기선을 상향 돌파)
            if prev_short <= prev_long && curr_short > curr_long {
                signals.push(PlotPoint::new(i as f64, curr_short)); // i를 x 값으로 사용
            }
            // 데드 크로스 (단기선이 장기선을 하향 돌파)
            else if prev_short >= prev_long && curr_short < curr_long {
                signals.push(PlotPoint::new(i as f64, curr_short)); // i를 x 값으로 사용
            }
        }

        signals
    }
    // fn update_target(&mut self) {
    //     self.target_value = rand::random::<f64>() * 2.0 - 1.0; // New random target between -1 and 1
    // }

    // fn interpolate_value(&mut self) {
    //     // Smoothly interpolate current_value towards target_value
    //     let difference = self.target_value - self.current_value;
    //     self.current_value += difference * 0.1; // Adjust this factor to control movement speed
    // }
    fn update_stock_data(&mut self, stock_name: &str) {
        let stock_type = "day".to_string();
        let new_data = StockData::get_data(stock_name, &stock_type);

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
                            if let (Some(last_value), Some(first_value)) =
                                (points.last(), points.first())
                            {
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

                    let button_text = RichText::new("Moving Averages").size(14.0).strong();
                    ui.menu_button(button_text, |ui| {
                        // 단순한 아래 화살표 사용
                        ui.set_min_width(150.0);

                        ui.horizontal(|ui| {
                            for period in &[MAPeriod::MA5, MAPeriod::MA10, MAPeriod::MA20] {
                                let mut active = *self.ma_states.get(period).unwrap_or(&false);
                                if ui.checkbox(&mut active, period.name()).clicked() {
                                    self.ma_states.insert(*period, active);
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            for period in &[MAPeriod::MA60, MAPeriod::MA224] {
                                let mut active = *self.ma_states.get(period).unwrap_or(&false);
                                if ui.checkbox(&mut active, period.name()).clicked() {
                                    self.ma_states.insert(*period, active);
                                }
                            }
                        });
                    });

                    ui.selectable_value(&mut self.time_frame, TimeFrame::Day, "1D");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Week, "1W");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Month, "1M");
                    ui.selectable_value(&mut self.time_frame, TimeFrame::Year, "1Y");
                });

                // Main Chart
                ui.group(|ui| {
                    let first_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .first_key_value()
                        .unwrap()
                        .0 as f64;
                    let last_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .last_key_value()
                        .unwrap()
                        .0 as f64;

                    let plot = egui_plot::Plot::new("stock_chart")
                        .height(500.0)
                        .width(800.)
                        .view_aspect(10.0) // 3.0에서 8.0으로 증가
                        .show_axes(true) // y축 숨기기
                        .auto_bounds(Vec2b::new(false, false)) // [x축 자동조절, y축 자동조절]
                        .include_x(first_value) // x축 끝점f
                        .include_x(last_value)
                        .include_y(0)
                        .include_y(250);

                    plot.show(ui, |plot_ui| {
                        if let Ok(measurements) = self.measurements.lock() {
                            match self.chart_type {
                                ChartType::Line => {
                                    let line_points: Vec<[f64; 2]> = measurements
                                        .values
                                        .iter()
                                        .map(|(i, candle)| [*i as f64, candle.close])
                                        .collect();
                                    // println!("{:?}", line_points[0]);
                                    plot_ui.line(
                                        egui_plot::Line::new(egui_plot::PlotPoints::new(
                                            line_points,
                                        ))
                                        .color(egui::Color32::from_rgb(0, 150, 255))
                                        .width(2.0),
                                    );
                                    // 추세선 추가
                                    // let n = measurements.values.len() as f64;
                                    // let points: Vec<(f64, f64)> = measurements
                                    //     .values
                                    //     .iter()
                                    //     .map(|(i, candle)| (*i as f64, candle.close))
                                    //     .collect();

                                    // let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
                                    // let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
                                    // let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
                                    // let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();

                                    // let m =
                                    //     (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
                                    // let b = (sum_y - m * sum_x) / n;

                                    // let first_x =
                                    //     *measurements.values.first_key_value().unwrap().0 as f64;
                                    // let last_x =
                                    //     *measurements.values.last_key_value().unwrap().0 as f64;

                                    // let trend_points =
                                    //     vec![[first_x, m * first_x + b], [last_x, m * last_x + b]];

                                    // plot_ui.line(
                                    //     egui_plot::Line::new(egui_plot::PlotPoints::new(
                                    //         trend_points,
                                    //     ))
                                    //     .color(egui::Color32::from_rgb(255, 100, 100))
                                    //     .width(2.0),
                                    // );
                                }
                                // Inside the match self.chart_type block, replace the Candle case with:
                                ChartType::Candle => {
                                    let candles: Vec<BoxElem> = measurements
                                        .values
                                        .iter()
                                        .map(|(i, candle)| {
                                            println!("{}", i);
                                            let lower_whisker = candle.low; // 최저가
                                            let upper_whisker = candle.high; // 최고가
                                            let lower_bound = candle.open.min(candle.close); // 몸체 아래쪽 경계
                                            let upper_bound = candle.open.max(candle.close); // 몸체 위쪽 경계
                                            let median = (candle.open + candle.close) / 2.0; // 중앙값

                                            let spread = BoxSpread::new(
                                                lower_whisker, // 최저가
                                                lower_bound,   // 몸체 아래쪽
                                                median,        // 중앙값
                                                upper_bound,   // 몸체 위쪽
                                                upper_whisker, // 최고가
                                            );

                                            let color = colors(candle.open,candle.close);

                                            BoxElem::new(*i as f64, spread)
                                                .fill(color)
                                                .stroke(egui::Stroke::new(2.0, color))
                                                .whisker_width(0.5) // 심지 너비
                                                .box_width(0.8) // 몸통 너비
                                        })
                                        .collect();

                                    let box_plot = egui_plot::BoxPlot::new(candles);
                                    plot_ui.box_plot(box_plot);
                                }
                            }
                            ctx.request_repaint();

                            // if self.show_ma {
                            if let PlotPoints::Owned(points) = measurements.plot_values() {
                                // 활성화된 각 이동평균선 표시
                                for (&period, &is_active) in &self.ma_states {
                                    if is_active {
                                        let ma =
                                            self.calculate_moving_average(&points, period.value());
                                        let color = match period {
                                            MAPeriod::MA5 => egui::Color32::from_rgb(255, 0, 0), // 빨강
                                            MAPeriod::MA10 => egui::Color32::from_rgb(0, 255, 0), // 초록
                                            MAPeriod::MA20 => egui::Color32::from_rgb(0, 0, 255), // 파랑
                                            MAPeriod::MA60 => egui::Color32::from_rgb(255, 165, 0), // 주황
                                            MAPeriod::MA224 => egui::Color32::from_rgb(128, 0, 128), // 보라
                                        };

                                        plot_ui.line(
                                            egui_plot::Line::new(PlotPoints::Owned(ma))
                                                .color(color)
                                                .width(1.5)
                                                .name(period.name()),
                                        );
                                    }
                                }
                            }
                        }
                        // }
                    });
                });

                ui.group(|ui| {
                    let first_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .first_key_value()
                        .unwrap()
                        .0 as f64; // 이미 정규화된 값 사용

                    let last_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .last_key_value()
                        .unwrap()
                        .0 as f64; // 이미 정규화된 값 사용

                    let plot = egui_plot::Plot::new("volume_chart")
                        .height(100.0)
                        .width(800.)
                        .view_aspect(5.0)
                        .auto_bounds(Vec2b::new(false, false))
                        .show_axes(false)
                        .include_x(first_value)
                        .include_x(last_value)
                        .include_y(0)
                        .include_y(500000000);

                    plot.show(ui, |plot_ui| {
                        if let Ok(measurements) = self.measurements.lock() {
                            let bars: Vec<Bar> = measurements
                                .values
                                .iter()
                                .map(|(i, candle)| {
                                    let color = if candle.close <= candle.open {
                                        egui::Color32::from_rgba_premultiplied(255, 59, 59, 255)
                                    } else {
                                        egui::Color32::from_rgba_premultiplied(66, 133, 255, 255)
                                    };

                                    let bar_width = if measurements.values.len() > 1 {
                                        (last_value - first_value)
                                            / measurements.values.len() as f64
                                            * 0.8
                                    } else {
                                        0.8
                                    };

                                    Bar::new(*i as f64, candle.volume / 10.) // 이미 정규화된 타임스탬프 사용
                                        .width(bar_width)
                                        .fill(color)
                                        .stroke(egui::Stroke::new(1.0, color))
                                })
                                .collect();

                            plot_ui.bar_chart(egui_plot::BarChart::new(bars));
                        }
                    });
                });
                // RSI Chart
                ui.group(|ui| {
                    let first_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .first_key_value()
                        .unwrap()
                        .0 as f64;
                    let last_value = *self
                        .measurements
                        .lock()
                        .unwrap()
                        .values
                        .last_key_value()
                        .unwrap()
                        .0 as f64;

                    let plot = egui_plot::Plot::new("rsi_chart")
                        .height(100.0)
                        .width(800.)
                        .view_aspect(5.0)
                        .auto_bounds(Vec2b::new(false, false))
                        .show_axes(true)
                        .include_x(first_value) // x축 끝점
                        .include_x(last_value)
                        .include_y(0)
                        .include_y(100)
                        .allow_boxed_zoom(false)
                        .allow_zoom(Vec2b::new(false, false))
                        .allow_scroll(Vec2b::new(true, false))
                        .allow_drag(Vec2b::new(false, false));

                    // RSI Chart 부분만 수정
                    plot.show(ui, |plot_ui| {
                        if let Ok(measurements) = self.measurements.lock() {
                            // RSI 계산 (이제 타임스탬프와 RSI 값 쌍의 벡터를 반환)
                            let rsi_values = self.calculate_rsi(&measurements.values);

                            // RSI 선 그리기 - 정규화된 타임스탬프 사용
                            let line_points: Vec<[f64; 2]> = rsi_values
                                .iter()
                                .map(|(timestamp, value)| [*timestamp, *value])
                                .collect();

                            plot_ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::new(line_points))
                                    .color(egui::Color32::from_rgb(255, 165, 0))
                                    .width(1.5),
                            );

                            // 기준선들 (변경 없음)
                            plot_ui.hline(
                                egui_plot::HLine::new(70.0)
                                    .color(egui::Color32::from_rgb(255, 0, 0))
                                    .width(1.0)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
                            );

                            plot_ui.hline(
                                egui_plot::HLine::new(30.0)
                                    .color(egui::Color32::from_rgb(0, 255, 0))
                                    .width(1.0)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
                            );

                            plot_ui.hline(
                                egui_plot::HLine::new(50.0)
                                    .color(egui::Color32::GRAY)
                                    .width(1.0)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
                            );
                        }
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
