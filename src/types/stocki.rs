use crate::types::egui::FontData;
use crate::types::egui::FontDefinitions;
use crate::types::BTreeMap;
use crate::types::ChartType;
use crate::types::Id;
use crate::types::LangType;
use crate::types::MAPeriod;
use crate::types::MeasurementWindow;
use crate::types::RichText;
use crate::types::SignalType;
use crate::types::StockData;
use crate::types::Stocki;
use crate::types::TimeFrame;
use crate::types::Vec2b;
use crate::utils::colors;
use eframe::egui;
use egui_plot::Bar;
use egui_plot::BoxElem;
use egui_plot::BoxSpread;
use egui_plot::PlotPoint;
use egui_plot::PlotPoints;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
impl Stocki {
    pub fn default(look_behind: usize) -> Self {
        let lang_type = LangType::Korean;
        let time_frame = TimeFrame::Day; // 기본값
        let selected_stock = match lang_type {
            LangType::English => "AAPL".to_string(),
            _ => "005930.KS".to_string(),
        };

        let new_data = StockData::get_data(&selected_stock, "1d", &lang_type);
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
            time_frame,
            previous_time_frame: time_frame, // 초기값은 현재 time_frame과 동일하게
            ma_states,
            chart_id: Id::new("stock_chart"),
            volume_id: Id::new("volme_chart"),
            rsi_id: Id::new("rsi_chart"),
            last_update: Instant::now(),
        }
    }

    fn should_update(&self) -> bool {
        let elapsed = self.last_update.elapsed();

        // TimeFrame에 따라 업데이트 간격 결정
        let update_interval = match self.time_frame {
            TimeFrame::Minute1 => std::time::Duration::from_secs(1),
            TimeFrame::Minute2 => std::time::Duration::from_secs(1),
            TimeFrame::Minute5 => std::time::Duration::from_secs(1),
            TimeFrame::Minute15 => std::time::Duration::from_secs(1),
            TimeFrame::Minute30 => std::time::Duration::from_secs(1),
            TimeFrame::Hour1 => std::time::Duration::from_secs(1),
            TimeFrame::Day => std::time::Duration::from_secs(1),
            TimeFrame::Week => std::time::Duration::from_secs(1),
            TimeFrame::Month => std::time::Duration::from_secs(1),
        };

        elapsed >= update_interval
    }
    pub fn calculate_moving_average(&self, prices: &[PlotPoint], period: usize) -> Vec<PlotPoint> {
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
    pub fn calculate_rsi(&self, prices: &BTreeMap<u64, StockData>) -> Vec<(f64, f64)> {
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

    pub fn show_signal_indicators(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Trading Signals: ");

            if let Ok(measurements) = self.measurements.lock() {
                if let PlotPoints::Owned(points) = measurements.plot_values() {
                    // 충분한 데이터가 있는지 먼저 확인
                    if points.len() < 60 {
                        // 최소 필요 데이터 포인트
                        ui.label("Insufficient data for signals");
                        return;
                    }

                    // 이동평균선 계산
                    let short_ma = self.calculate_moving_average(&points, 20);
                    let long_ma = self.calculate_moving_average(&points, 60);

                    // 안전하게 인덱스 확인
                    let signal = if short_ma.len() >= 2 && long_ma.len() >= 2 {
                        let last_idx = (short_ma.len() - 1).min(long_ma.len() - 1);
                        let prev_idx = last_idx.saturating_sub(1);

                        let prev_short = short_ma[prev_idx].y;
                        let prev_long = long_ma[prev_idx].y;
                        let curr_short = short_ma[last_idx].y;
                        let curr_long = long_ma[last_idx].y;

                        // 매매 신호 생성
                        if prev_short <= prev_long && curr_short > curr_long {
                            Some(SignalType::Buy)
                        } else if prev_short >= prev_long && curr_short < curr_long {
                            Some(SignalType::Sell)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // RSI 계산 및 신호 생성
                    let rsi = if let Some(last_rsi) = self
                        .calculate_rsi(&measurements.values)
                        .last()
                        .filter(|&&(_timestamp, value)| !value.is_nan())
                    // NaN 값 필터링
                    {
                        if last_rsi.1 < 30.0 {
                            Some(SignalType::BuyRSI)
                        } else if last_rsi.1 > 70.0 {
                            Some(SignalType::SellRSI)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // MA 신호 표시
                    ui.group(|ui| {
                        ui.label("MA Cross:");
                        match signal {
                            Some(SignalType::Buy) => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("⬆ 매수")
                                        .color(egui::Color32::from_rgb(0, 255, 0))
                                        .strong(),
                                ));
                            }
                            Some(SignalType::Sell) => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("⬇ 매도")
                                        .color(egui::Color32::from_rgb(255, 0, 0))
                                        .strong(),
                                ));
                            }
                            _ => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("대기")  // NEUTRAL 대신 "대기"
                                        .color(egui::Color32::GRAY)
                                ));
                            }
                        }
                    });

                    // RSI 신호 표시
                    ui.group(|ui| {
                        ui.label("RSI:");
                        match rsi {
                            Some(SignalType::BuyRSI) => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("과매도 - 매수 기회")
                                        .color(egui::Color32::from_rgb(0, 255, 0))
                                        .strong(),
                                ));
                            }
                            Some(SignalType::SellRSI) => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("과매수 - 매도 기회")
                                        .color(egui::Color32::from_rgb(255, 0, 0))
                                        .strong(),
                                ));
                            }
                            _ => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("적정 수준")  // NEUTRAL 대신 "적정 수준"
                                        .color(egui::Color32::GRAY)
                                ));
                            }
                        }
                    });
                }
            }
        });
    }
    pub fn update_stock_data(&mut self) {
        let stock_type = self.time_frame.to_string();
        let lang_type = &self.lang_type;

        // 타임프레임이 변경된 경우에만 전체 데이터를 새로 가져옴
        if self.time_frame != self.previous_time_frame {
            println!(
                "TimeFrame changed: {:?} -> {:?}",
                self.previous_time_frame, self.time_frame
            );
            let new_data = StockData::get_data(&self.selected_stock, &stock_type, lang_type);

            if let Ok(mut measurements) = self.measurements.lock() {
                *measurements = MeasurementWindow::new_with_look_behind(1000, new_data);
            }
            self.previous_time_frame = self.time_frame;
        } else {
            // 그외의 경우는 마지막 타임스탬프 이후의 데이터만 가져옴
            if let Ok(measurements) = self.measurements.lock() {
                if let Some(&last_timestamp) = measurements.values.keys().last() {
                    if let Some(new_data) = StockData::get_latest_data(
                        &self.selected_stock,
                        &stock_type,
                        lang_type,
                        last_timestamp,
                    ) {
                        drop(measurements); // 먼저 lock 해제

                        // 새로운 데이터만 추가
                        if let Ok(mut measurements) = self.measurements.lock() {
                            for (timestamp, data) in new_data {
                                measurements.values.insert(timestamp, data);
                            }
                        }
                    }
                }
            }
        }

        self.last_update = Instant::now();
        self.chart_id = Id::new(format!("stock_chart_{}", rand::random::<u64>()));
        self.volume_id = Id::new(format!("volme_chart_{}", rand::random::<u64>()));
        self.rsi_id = Id::new(format!("rsi_chart_{}", rand::random::<u64>()));
    }

    pub fn draw_chart(&self, ui: &mut egui::Ui) {
        if let Ok(measurements) = self.measurements.lock() {
            if let (Some((&first_key, _)), Some((&last_key, _))) = (
                measurements.values.first_key_value(),
                measurements.values.last_key_value(),
            ) {
                let first_value = first_key as f64;
                let last_value = last_key as f64;
                // println!("{}", measurements.values.len());
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
                    .width(780.)
                    .view_aspect(10.0)
                    .show_axes(false)
                    .auto_bounds(Vec2b::new(false, false))
                    .include_x(first_value)
                    .include_x(last_value + 2.)
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
    pub fn draw_volume_chart(&self, ui: &mut egui::Ui) {
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
                    .width(780.)
                    .view_aspect(5.0)
                    .auto_bounds(Vec2b::new(false, false))
                    .show_axes(false)
                    .include_x(first_value)
                    .include_x(last_value + 2.)
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

    pub fn draw_rsi_chart(&self, ui: &mut egui::Ui) {
        if let Ok(measurements) = self.measurements.lock() {
            if let (Some((&first_key, _)), Some((&last_key, _))) = (
                measurements.values.first_key_value(),
                measurements.values.last_key_value(),
            ) {
                let first_value = first_key as f64;
                let last_value = last_key as f64;

                let plot = egui_plot::Plot::new(self.rsi_id) // 동적 ID 사용
                    .height(100.0)
                    .width(780.)
                    .view_aspect(5.0)
                    .auto_bounds(Vec2b::new(false, false))
                    .show_axes(true)
                    .include_x(first_value)
                    .include_x(last_value + 2.)
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
                        (70.0, egui::Color32::from_rgb(255, 0, 0)), // 상단 경계선
                        (30.0, egui::Color32::from_rgb(0, 255, 0)), // 하단 경계선
                        (50.0, egui::Color32::GRAY),                // 중간선
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
impl eframe::App for Stocki {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut font_definitions = FontDefinitions::default();
        font_definitions.font_data.insert(
            "my_font".to_owned(),
            FontData::from_static(include_bytes!("../../assets/font/NanumGothic-Bold.ttf")),
        );
        font_definitions
            .families
            .insert(egui::FontFamily::Proportional, vec!["my_font".to_owned()]);
        ctx.set_fonts(font_definitions);
        if self.should_update() {
            self.update_stock_data();
        }
        let now: Instant = Instant::now();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.show_signal_indicators(ui);

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
                                self.selected_stock =
                                    self.stocks.first().cloned().unwrap_or_default();

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
                    // lock 불필요, 직접 참조
                    let mut selected = None;

                    ui.menu_button(&self.selected_stock, |ui| {
                        // 직접 selected_stock 사용
                        for stock in &self.stocks {
                            if ui.button(stock).clicked() {
                                selected = Some(stock.clone());
                                ui.close_menu();
                            }
                        }
                    });
                    if let Some(selected_stock) = selected {
                        self.selected_stock = selected_stock;
                        self.lang_type = LangType::English; // 미국 주식 선택시 언어 변경
                        self.time_frame = TimeFrame::Day; // 타임프레임 초기화
                        self.previous_time_frame = TimeFrame::Day;

                        // 강제 업데이트
                        let stock_type = self.time_frame.to_string();
                        let new_data =
                            StockData::get_data(&self.selected_stock, &stock_type, &self.lang_type);

                        if let Ok(mut measurements) = self.measurements.lock() {
                            *measurements = MeasurementWindow::new_with_look_behind(1000, new_data);
                        }

                        self.last_update = Instant::now();
                    }
                    // if let Some(selected_stock) = selected {
                    //     self.selected_stock = selected_stock; // clone 불필요
                    //     self.update_stock_data();
                    // }
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
                            TimeFrame::Minute1,
                            TimeFrame::Minute2,
                            TimeFrame::Minute5,
                            TimeFrame::Minute30,
                            TimeFrame::Hour1,
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
                                self.time_frame = timeframe; // 새로운 타임프레임 설정
                                ctx.request_repaint();
                                ui.close_menu();
                            }
                            // if ui
                            //     .selectable_label(
                            //         self.time_frame == timeframe,
                            //         timeframe.display_name(),
                            //     )
                            //     .clicked()
                            // {
                            //     // 타임프레임이 변경되면 즉시 업데이트
                            //     self.time_frame = timeframe;
                            //     let stock_name = self.selected_stock.lock().unwrap().clone();
                            //     self.update_stock_data(&stock_name);
                            //     ctx.request_repaint(); // 강제로 화면 갱신
                            //     ui.close_menu();
                            // }
                        }
                    });

                    if let Some(new_timeframe) = selected_timeframe {
                        if new_timeframe != self.time_frame {
                            self.time_frame = new_timeframe;
                            self.update_stock_data(); // 내부에서 직접 selected_stock 접근
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
