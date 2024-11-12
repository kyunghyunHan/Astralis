#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example
use eframe::egui::{self, Id, RichText, Vec2b};
use egui::{CentralPanel, Context, FontData, FontDefinitions, SidePanel, TopBottomPanel};
use egui_plot::{Bar, BoxElem, BoxSpread, PlotPoint, PlotPoints};
use std::collections::BTreeMap;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use stocki::{
    types::{ChartType, LangType, MAPeriod, MeasurementWindow, StockData, TimeFrame},
    utils::colors,
};

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
    selected_stock: Arc<Mutex<String>>,
    stocks: Vec<String>,
    chart_type: ChartType,
    lang_type: LangType,
    time_frame: TimeFrame,
    ma_states: std::collections::HashMap<MAPeriod, bool>,
    chart_id: Id,  // 차트 ID 추가
    volume_id: Id, // 차트 ID 추가
    rsi_id: Id,    // 차트 ID 추가
}

impl Stocki {
    fn default(look_behind: usize) -> Self {
        let lang_type = LangType::Korean;

        let selected_stock = match lang_type {
            LangType::English => Arc::new(Mutex::new("AAPL".to_string())),
            _ => Arc::new(Mutex::new("005930.KS".to_string())),
        };
        let selected_stock_clone = Arc::clone(&selected_stock);
        let stock_name = selected_stock_clone.lock().unwrap().clone();
        let new_data = StockData::get_data(&stock_name, "1d", &lang_type);
        let stocks = match lang_type {
            LangType::English => {
                vec![
                    "AAPL".to_string(),
                    "GOOGL".to_string(),
                    "MSFT".to_string(),
                    "AMZN".to_string(),
                    "META".to_string(),
                    "TSLA".to_string(),
                    "NVDA".to_string(),
                ]
            }
            _ => {
                vec!["005930.KS".to_string()]
            }
        };

        let mut ma_states = std::collections::HashMap::new();
        for period in [
            MAPeriod::MA5,
            MAPeriod::MA10,
            MAPeriod::MA20,
            MAPeriod::MA60,
            MAPeriod::MA224,
        ] {
            ma_states.insert(period, false);
        }

        Self {
            measurements: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(
                look_behind,
                new_data,
            ))),
            selected_stock,
            stocks,
            chart_type: ChartType::Line,
            lang_type: LangType::Korean,
            time_frame: TimeFrame::Day,
            ma_states,
            chart_id: Id::new("stock_chart"),  // 차트 ID 초기화
            volume_id: Id::new("volme_chart"), // 차트 ID 초기화
            rsi_id: Id::new("rsi_chart"),      // 차트 ID 초기화
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
    fn update_stock_data(&mut self, stock_name: &str) {
        let stock_type = self.time_frame.to_string();
        let lang_type = &self.lang_type;

        println!("Updating data for timeframe: {}", stock_type); // 디버그용
        let new_data = StockData::get_data(stock_name, stock_type, lang_type);

        if let Ok(mut measurements) = self.measurements.lock() {
            // 기존 데이터를 완전히 새로운 데이터로 교체
            *measurements = MeasurementWindow::new_with_look_behind(1000, new_data);
        }
        self.chart_id = Id::new(format!("stock_chart_{}", rand::random::<u64>()));
        self.volume_id = Id::new(format!("volme_chart_{}", rand::random::<u64>()));
        self.rsi_id = Id::new(format!("rsi_chart_{}", rand::random::<u64>()));
    }
    fn draw_chart(&self, ui: &mut egui::Ui) {
        if let Ok(measurements) = self.measurements.lock() {
            if let (Some((&first_key, _)), Some((&last_key, _))) = (
                measurements.values.first_key_value(),
                measurements.values.last_key_value(),
            ) {
                let first_value = first_key as f64;
                let last_value = last_key as f64;
                println!("{}", measurements.values.len());
                let (max_price, min_price) = measurements
                    .values
                    .values()
                    .fold((f64::MIN, f64::MAX), |acc, stock_data| {
                        (acc.0.max(stock_data.high), acc.1.min(stock_data.low))
                    });
                let price_range = max_price - min_price;
                let padding = price_range * 0.1;
                let min_y = (min_price - padding).max(0.0); // 최소값은 0 이상
                let max_y = max_price + padding;
                let plot = egui_plot::Plot::new(self.chart_id) // 동적 ID 사용
                    .height(500.0)
                    .width(800.)
                    .view_aspect(10.0)
                    .show_axes(false)
                    .auto_bounds(Vec2b::new(false, false))
                    .include_x(first_value)
                    .include_x(last_value)
                    .include_y(min_y)
                    .include_y(max_y);

                // 나머지 차트 그리기 코드...
                plot.show(ui, |plot_ui| {
                    // 기존 차트 그리기 로직...
                    match self.chart_type {
                        ChartType::Line => {
                            let line_points: Vec<[f64; 2]> = measurements
                                .values
                                .iter()
                                .map(|(i, candle)| [*i as f64, candle.close])
                                .collect();

                            plot_ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::new(line_points))
                                    .color(egui::Color32::from_rgb(0, 150, 255))
                                    .width(2.0),
                            );
                        }
                        ChartType::Candle => {
                            let candles: Vec<BoxElem> = measurements
                                .values
                                .iter()
                                .map(|(i, candle)| {
                                    let lower_whisker = candle.low;
                                    let upper_whisker = candle.high;
                                    let lower_bound = candle.open.min(candle.close);
                                    let upper_bound = candle.open.max(candle.close);
                                    let median = (candle.open + candle.close) / 2.0;

                                    let spread = BoxSpread::new(
                                        lower_whisker,
                                        lower_bound,
                                        median,
                                        upper_bound,
                                        upper_whisker,
                                    );

                                    let color = colors(candle.open, candle.close);

                                    BoxElem::new(*i as f64, spread)
                                        .fill(color)
                                        .stroke(egui::Stroke::new(2.0, color))
                                        .whisker_width(0.5)
                                        .box_width(0.8)
                                })
                                .collect();

                            let box_plot = egui_plot::BoxPlot::new(candles);
                            plot_ui.box_plot(box_plot);
                        }
                    }

                    if let PlotPoints::Owned(points) = measurements.plot_values() {
                        for (&period, &is_active) in &self.ma_states {
                            if is_active {
                                let ma = self.calculate_moving_average(&points, period.value());
                                let color = match period {
                                    MAPeriod::MA5 => egui::Color32::from_rgb(255, 0, 0),
                                    MAPeriod::MA10 => egui::Color32::from_rgb(0, 255, 0),
                                    MAPeriod::MA20 => egui::Color32::from_rgb(0, 0, 255),
                                    MAPeriod::MA60 => egui::Color32::from_rgb(255, 165, 0),
                                    MAPeriod::MA224 => egui::Color32::from_rgb(128, 0, 128),
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
                });
            }
        }
    }
    fn draw_volume_chart(&self, ui: &mut egui::Ui) {
        if let Ok(measurements) = self.measurements.lock() {
            if let (Some((&first_key, _)), Some((&last_key, _))) = (
                measurements.values.first_key_value(),
                measurements.values.last_key_value(),
            ) {
                let first_value = first_key as f64;
                let last_value = last_key as f64;

                // 최대 거래량 계산
                let max_volume = measurements
                    .values
                    .values()
                    .map(|data| data.volume)
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0);

                let volume_padding = max_volume * 0.1; // 10% 패딩 추가

                let plot = egui_plot::Plot::new(self.volume_id) // 동적 ID 사용
                    .height(100.0)
                    .width(800.)
                    .view_aspect(5.0)
                    .auto_bounds(Vec2b::new(false, false))
                    .show_axes(false)
                    .include_x(first_value)
                    .include_x(last_value)
                    .include_y(0.0)
                    .include_y(max_volume + volume_padding)
                    .allow_boxed_zoom(false)
                    .allow_zoom(Vec2b::new(false, false))
                    .allow_scroll(Vec2b::new(true, false))
                    .allow_drag(Vec2b::new(false, false));

                plot.show(ui, |plot_ui| {
                    let bars: Vec<Bar> = measurements
                        .values
                        .iter()
                        .map(|(i, candle)| {
                            // 색상 결정
                            let color = if candle.close <= candle.open {
                                egui::Color32::from_rgba_premultiplied(255, 59, 59, 255)
                            // 하락봉
                            } else {
                                egui::Color32::from_rgba_premultiplied(66, 133, 255, 255)
                                // 상승봉
                            };

                            // 바 너비 계산
                            let bar_width = if measurements.values.len() > 1 {
                                (last_value - first_value) / measurements.values.len() as f64 * 0.8
                            } else {
                                0.8
                            };

                            Bar::new(*i as f64, candle.volume)
                                .width(bar_width)
                                .fill(color)
                                .stroke(egui::Stroke::new(1.0, color))
                        })
                        .collect();

                    plot_ui.bar_chart(egui_plot::BarChart::new(bars));
                });
            }
        }
    }
    
fn draw_rsi_chart(&self, ui: &mut egui::Ui) {
    if let Ok(measurements) = self.measurements.lock() {
        if let (Some((&first_key, _)), Some((&last_key, _))) = (
            measurements.values.first_key_value(),
            measurements.values.last_key_value(),
        ) {
            let first_value = first_key as f64;
            let last_value = last_key as f64;

            let plot = egui_plot::Plot::new(self.rsi_id) // 동적 ID 사용
            .height(100.0)
                .width(800.)
                .view_aspect(5.0)
                .auto_bounds(Vec2b::new(false, false))
                .show_axes(true)
                .include_x(first_value)
                .include_x(last_value)
                .include_y(0)
                .include_y(100)
                .allow_boxed_zoom(false)
                .allow_zoom(Vec2b::new(false, false))
                .allow_scroll(Vec2b::new(true, false))
                .allow_drag(Vec2b::new(false, false));

            plot.show(ui, |plot_ui| {
                // RSI 계산 및 선 그리기
                let rsi_values = self.calculate_rsi(&measurements.values);
                let line_points: Vec<[f64; 2]> = rsi_values
                    .iter()
                    .map(|(timestamp, value)| [*timestamp, *value])
                    .collect();

                plot_ui.line(
                    egui_plot::Line::new(egui_plot::PlotPoints::new(line_points))
                        .color(egui::Color32::from_rgb(255, 165, 0))
                        .width(1.5),
                );

                // 기준선 그리기
                let reference_lines = [
                    (70.0, egui::Color32::from_rgb(255, 0, 0)),   // 상단 경계선
                    (30.0, egui::Color32::from_rgb(0, 255, 0)),   // 하단 경계선
                    (50.0, egui::Color32::GRAY),                  // 중간선
                ];

                for (value, color) in reference_lines.iter() {
                    plot_ui.hline(
                        egui_plot::HLine::new(*value)
                            .color(*color)
                            .width(1.0)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
                    );
                }
            });
        }
    }
}
}
////
impl eframe::App for Stocki {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut font_definitions = FontDefinitions::default();
        font_definitions.font_data.insert(
            "my_font".to_owned(),
            FontData::from_static(include_bytes!("../assets/font/NanumGothic-Bold.ttf")),
        );
        font_definitions
            .families
            .insert(egui::FontFamily::Proportional, vec!["my_font".to_owned()]);
        ctx.set_fonts(font_definitions);
        let now: Instant = Instant::now();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::menu::bar(ui, |ui| {
                    if !cfg!(target_arch = "wasm32") {
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
                    ui.menu_button(RichText::new("aa").size(14.0), |ui| {
                        ui.set_min_width(100.0);
                        let arr = [LangType::English, LangType::Korean];

                        for timeframe in arr {
                            let response = ui.selectable_label(
                                self.lang_type == timeframe,
                                format!("{:?}", timeframe),
                            );

                            if response.clicked() {
                                // 언어 타입 변경
                                self.lang_type = timeframe;

                                // 언어에 따른 주식 리스트 업데이트
                                self.stocks = match self.lang_type {
                                    LangType::English => {
                                        vec![
                                            "AAPL".to_string(),
                                            "GOOGL".to_string(),
                                            "MSFT".to_string(),
                                            "AMZN".to_string(),
                                            "META".to_string(),
                                            "TSLA".to_string(),
                                            "NVDA".to_string(),
                                        ]
                                    }
                                    _ => {
                                        vec!["005930.KS".to_string()]
                                    }
                                };

                                // 선택된 주식 초기화 또는 첫 번째 주식으로 설정
                                if let Ok(mut selected) = self.selected_stock.lock() {
                                    *selected = self.stocks.first().cloned().unwrap_or_default();
                                }

                                // UI 갱신 요청
                                ctx.request_repaint();
                                ui.close_menu();
                            }
                        }
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                                        TimeFrame::Minute1 => "1m",
                                        TimeFrame::Minute2 => "2m",
                                        TimeFrame::Minute5 => "5m",
                                        TimeFrame::Minute15 => "15m",
                                        TimeFrame::Minute30 => "30m",
                                        TimeFrame::Hour1 => "60m",
                                        TimeFrame::Day => "1d",
                                        TimeFrame::Week => "1wk",
                                        TimeFrame::Month => "1mo",
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

                    let button_text = RichText::new("이동평균선").size(14.0).strong();
                    ui.menu_button(button_text, |ui| {
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
                    // Time Frame Dropdown
                    ui.add_space(32.0);
                    let current_timeframe = self.time_frame.display_name();
                    let mut selected_timeframe = None;

                    ui.menu_button(RichText::new(current_timeframe).size(14.0), |ui| {
                        ui.set_min_width(100.0);
                        let time_frames = [
                            TimeFrame::Day,
                            TimeFrame::Week,
                            TimeFrame::Month,
                            TimeFrame::Hour1,
                            TimeFrame::Minute1,
                            TimeFrame::Minute2,
                            TimeFrame::Minute5,
                            TimeFrame::Minute30,
                            // TimeFrame::Minute30,
                        ];

                        for &timeframe in &time_frames {
                            if ui
                                .selectable_label(
                                    self.time_frame == timeframe,
                                    timeframe.display_name(),
                                )
                                .clicked()
                            {
                                // 타임프레임이 변경되면 즉시 업데이트
                                self.time_frame = timeframe;
                                let stock_name = self.selected_stock.lock().unwrap().clone();
                                self.update_stock_data(&stock_name);
                                ctx.request_repaint(); // 강제로 화면 갱신
                                ui.close_menu();
                            }
                        }
                    });

                    if let Some(new_timeframe) = selected_timeframe {
                        if new_timeframe != self.time_frame {
                            self.time_frame = new_timeframe;
                            let stock_name = self.selected_stock.lock().unwrap().clone();
                            self.update_stock_data(&stock_name);
                        }
                    }
                });
                ui.group(|ui| {
                    self.draw_chart(ui);
                });
                ui.group(|ui| {
                    self.draw_volume_chart(ui);
                });
                ui.group(|ui| {
                    self.draw_rsi_chart(ui);
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
