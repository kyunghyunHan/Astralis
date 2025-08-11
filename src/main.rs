#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, BoxPlot, BoxElem, BoxSpread};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Crypto Chart"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Crypto Chart",
        options,
        Box::new(|_cc| Ok(Box::<CryptoApp>::default())),
    )
}

#[derive(Clone, Debug)]
struct CandleData {
    timestamp: f64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Clone, PartialEq)]
enum ChartType {
    Line,
    Candlestick,
}

#[derive(Clone, PartialEq)]
enum Timeframe {
    M1,   // 1분
    M3,   // 3분
    M5,   // 5분
    M15,  // 15분
    M30,  // 30분
    H1,   // 1시간
    H4,   // 4시간
    H12,  // 12시간
    D1,   // 일봉
    W1,   // 주봉
    MN1,  // 월봉
}

impl Timeframe {
    fn to_api_string(&self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M3 => "3m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::M30 => "30m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::H12 => "12h",
            Timeframe::D1 => "1d",
            Timeframe::W1 => "1w",
            Timeframe::MN1 => "1M",
        }
    }
    
    fn to_display_string(&self) -> &'static str {
        match self {
            Timeframe::M1 => "1분",
            Timeframe::M3 => "3분",
            Timeframe::M5 => "5분",
            Timeframe::M15 => "15분",
            Timeframe::M30 => "30분",
            Timeframe::H1 => "1시간",
            Timeframe::H4 => "4시간",
            Timeframe::H12 => "12시간",
            Timeframe::D1 => "일봉",
            Timeframe::W1 => "주봉",
            Timeframe::MN1 => "월봉",
        }
    }
    
    fn get_window_size(&self) -> f64 {
        match self {
            Timeframe::M1 => 60.0 * 100.0,      // 100분 = 1.67시간
            Timeframe::M3 => 60.0 * 300.0,      // 300분 = 5시간
            Timeframe::M5 => 60.0 * 500.0,      // 500분 = 8.33시간
            Timeframe::M15 => 60.0 * 1500.0,    // 1500분 = 25시간
            Timeframe::M30 => 60.0 * 3000.0,    // 3000분 = 50시간
            Timeframe::H1 => 60.0 * 60.0 * 100.0,   // 100시간
            Timeframe::H4 => 60.0 * 60.0 * 400.0,   // 400시간
            Timeframe::H12 => 60.0 * 60.0 * 1200.0, // 1200시간 = 50일
            Timeframe::D1 => 60.0 * 60.0 * 24.0 * 100.0, // 100일
            Timeframe::W1 => 60.0 * 60.0 * 24.0 * 7.0 * 50.0, // 50주
            Timeframe::MN1 => 60.0 * 60.0 * 24.0 * 30.0 * 12.0, // 12개월
        }
    }
}

struct CryptoApp {
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
    chart_type: ChartType,
    timeframe: Timeframe,
    candle_width: f64,
    is_loading: bool,
    runtime: Option<tokio::runtime::Runtime>,
    data_receiver: Option<mpsc::UnboundedReceiver<Vec<CandleData>>>,
    latest_timestamp: f64,
    view_window_start: f64,
    window_size: f64,
    is_dragging: bool,
    is_live_mode: bool,
    timeframe_changed: bool,
}

impl Default for CryptoApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let timeframe = Timeframe::M1;
        let window_size = timeframe.get_window_size();
        
        let app = Self {
            candle_data: Arc::new(Mutex::new(VecDeque::new())),
            chart_type: ChartType::Candlestick,
            timeframe,
            candle_width: 0.8,
            is_loading: true,
            runtime: Some(tokio::runtime::Runtime::new().unwrap()),
            data_receiver: Some(rx),
            latest_timestamp: 0.0,
            view_window_start: 0.0,
            window_size,
            is_dragging: false,
            is_live_mode: true,
            timeframe_changed: false,
        };
        
        // Start fetching data
        if let Some(rt) = &app.runtime {
            let candle_data_clone = app.candle_data.clone();
            let timeframe_clone = app.timeframe.clone();
            rt.spawn(fetch_binance_data(tx, candle_data_clone, timeframe_clone));
        }
        
        app
    }
}

async fn fetch_binance_data(
    tx: mpsc::UnboundedSender<Vec<CandleData>>,
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
    timeframe: Timeframe,
) {
    loop {
        // 현재 데이터 범위 확인
        let (_current_start, _current_end) = if let Ok(data) = candle_data.lock() {
            if let (Some(first), Some(last)) = (data.front(), data.back()) {
                (first.timestamp, last.timestamp)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };
        
        // 최신 500개 데이터만 업데이트 (실시간 업데이트용)
        match fetch_klines_latest(&timeframe).await {
            Ok(candles) => {
                if let Ok(mut data) = candle_data.lock() {
                    if data.is_empty() {
                        // 처음 로딩일 때만 전체 데이터 추가
                        data.extend(candles.iter().cloned());
                    } else {
                        // 기존 데이터가 있으면 최신 부분만 업데이트
                        let latest_existing_time = data.back().map(|d| d.timestamp).unwrap_or(0.0);
                        
                        for new_candle in &candles {
                            if new_candle.timestamp > latest_existing_time {
                                data.push_back(new_candle.clone());
                            } else if let Some(existing_pos) = data.iter().position(|existing| 
                                (existing.timestamp - new_candle.timestamp).abs() < 1.0) {
                                data[existing_pos] = new_candle.clone();
                            }
                        }
                        
                        // 메모리 관리: 너무 많은 데이터 제거 (최대 10000개 유지)
                        while data.len() > 10000 {
                            data.pop_front();
                        }
                    }
                }
                
                // Send to UI
                if tx.send(candles).is_err() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error fetching data: {}", e);
            }
        }
        
        // 시간봉에 따라 업데이트 주기 조정
        let update_interval = match timeframe {
            Timeframe::M1 | Timeframe::M3 | Timeframe::M5 => 5,  // 5초
            Timeframe::M15 | Timeframe::M30 => 30,               // 30초
            Timeframe::H1 | Timeframe::H4 => 60,                 // 1분
            _ => 300,                                            // 5분
        };
        
        tokio::time::sleep(tokio::time::Duration::from_secs(update_interval)).await;
    }
}

async fn fetch_klines_latest(timeframe: &Timeframe) -> Result<Vec<CandleData>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval={}&limit=500",
        timeframe.to_api_string()
    );
    
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }
    
    let text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    
    let mut candles = Vec::new();
    
    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(kline_array) = item.as_array() {
                if kline_array.len() >= 11 {
                    let timestamp = kline_array[0].as_i64().unwrap_or(0) as f64;
                    let open = kline_array[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let high = kline_array[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let low = kline_array[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let close = kline_array[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let volume = kline_array[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    
                    if open > 0.0 && high > 0.0 && low > 0.0 && close > 0.0 {
                        candles.push(CandleData {
                            timestamp: timestamp / 1000.0,
                            open,
                            high,
                            low,
                            close,
                            volume,
                        });
                    }
                }
            }
        }
    }
    
    Ok(candles)
}

async fn fetch_klines_for_range(
    start_time: f64, 
    end_time: f64,
    timeframe: &Timeframe,
) -> Result<Vec<CandleData>, Box<dyn std::error::Error>> {
    let start_ms = (start_time * 1000.0) as i64;
    let end_ms = (end_time * 1000.0) as i64;
    
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval={}&startTime={}&endTime={}&limit=1000",
        timeframe.to_api_string(), start_ms, end_ms
    );
    
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }
    
    let text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    
    let mut candles = Vec::new();
    
    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(kline_array) = item.as_array() {
                if kline_array.len() >= 11 {
                    let timestamp = kline_array[0].as_i64().unwrap_or(0) as f64;
                    let open = kline_array[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let high = kline_array[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let low = kline_array[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let close = kline_array[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    let volume = kline_array[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                    
                    if open > 0.0 && high > 0.0 && low > 0.0 && close > 0.0 {
                        candles.push(CandleData {
                            timestamp: timestamp / 1000.0,
                            open,
                            high,
                            low,
                            close,
                            volume,
                        });
                    }
                }
            }
        }
    }
    
    Ok(candles)
}

impl eframe::App for CryptoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for new data - only when not dragging
        if !self.is_dragging {
            if let Some(receiver) = &mut self.data_receiver {
                while let Ok(new_candles) = receiver.try_recv() {
                    if !new_candles.is_empty() {
                        self.is_loading = false;
                        
                        let old_latest = self.latest_timestamp;
                        
                        // Update latest timestamp
                        if let Some(latest) = new_candles.last() {
                            self.latest_timestamp = latest.timestamp;
                            
                            if self.view_window_start == 0.0 {
                                self.view_window_start = self.latest_timestamp - self.window_size;
                                self.is_live_mode = true;
                            } else if self.is_live_mode {
                                // 라이브 모드일 때: 최신 데이터가 윈도우 오른쪽 끝에서 약간 안쪽에 오도록
                                let buffer = match self.timeframe {
                                    Timeframe::M1 => 60.0 * 5.0,     // 5분 버퍼
                                    Timeframe::M3 => 60.0 * 15.0,    // 15분 버퍼
                                    Timeframe::M5 => 60.0 * 25.0,    // 25분 버퍼
                                    Timeframe::M15 => 60.0 * 75.0,   // 75분 버퍼
                                    Timeframe::M30 => 60.0 * 150.0,  // 150분 버퍼
                                    Timeframe::H1 => 60.0 * 60.0 * 5.0, // 5시간 버퍼
                                    Timeframe::H4 => 60.0 * 60.0 * 20.0, // 20시간 버퍼
                                    _ => 60.0 * 60.0 * 24.0 * 5.0,   // 5일 버퍼
                                };
                                self.view_window_start = self.latest_timestamp + buffer - self.window_size;
                            }
                        }
                    }
                }
            }
        }
        
        // Simple top controls
        egui::TopBottomPanel::top("control_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 시간봉 선택
                ui.label("시간봉:");
                let old_timeframe = self.timeframe.clone();
                egui::ComboBox::from_id_salt("timeframe")
                    .selected_text(self.timeframe.to_display_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.timeframe, Timeframe::M1, "1분");
                        ui.selectable_value(&mut self.timeframe, Timeframe::M3, "3분");
                        ui.selectable_value(&mut self.timeframe, Timeframe::M5, "5분");
                        ui.selectable_value(&mut self.timeframe, Timeframe::M15, "15분");
                        ui.selectable_value(&mut self.timeframe, Timeframe::M30, "30분");
                        ui.selectable_value(&mut self.timeframe, Timeframe::H1, "1시간");
                        ui.selectable_value(&mut self.timeframe, Timeframe::H4, "4시간");
                        ui.selectable_value(&mut self.timeframe, Timeframe::H12, "12시간");
                        ui.selectable_value(&mut self.timeframe, Timeframe::D1, "일봉");
                        ui.selectable_value(&mut self.timeframe, Timeframe::W1, "주봉");
                        ui.selectable_value(&mut self.timeframe, Timeframe::MN1, "월봉");
                    });
                
                // 시간봉이 변경되었는지 확인
                if old_timeframe != self.timeframe {
                    self.timeframe_changed = true;
                    self.window_size = self.timeframe.get_window_size();
                    self.is_loading = true;
                    
                    // 데이터 초기화
                    if let Ok(mut data) = self.candle_data.lock() {
                        data.clear();
                    }
                    
                    // 새로운 시간봉으로 데이터 가져오기
                    if let Some(rt) = &self.runtime {
                        let (tx, rx) = mpsc::unbounded_channel();
                        self.data_receiver = Some(rx);
                        
                        let candle_data_clone = self.candle_data.clone();
                        let timeframe_clone = self.timeframe.clone();
                        rt.spawn(fetch_binance_data(tx, candle_data_clone, timeframe_clone));
                    }
                    
                    // 뷰 리셋
                    self.view_window_start = 0.0;
                    self.is_live_mode = true;
                }
                
                ui.separator();
                
                ui.label("차트 타입:");
                egui::ComboBox::from_id_salt("chart_type")
                    .selected_text(match self.chart_type {
                        ChartType::Line => "라인",
                        ChartType::Candlestick => "캔들스틱",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.chart_type, ChartType::Line, "라인");
                        ui.selectable_value(&mut self.chart_type, ChartType::Candlestick, "캔들스틱");
                    });
                
                if self.chart_type == ChartType::Candlestick {
                    ui.separator();
                    ui.label("캔들 너비:");
                    ui.add(egui::Slider::new(&mut self.candle_width, 0.1..=2.0).text(""));
                }
                
                ui.separator();
                
                if ui.button("라이브").clicked() {
                    // 라이브 모드로 전환
                    let buffer = match self.timeframe {
                        Timeframe::M1 => 60.0 * 5.0,
                        Timeframe::M3 => 60.0 * 15.0,
                        Timeframe::M5 => 60.0 * 25.0,
                        Timeframe::M15 => 60.0 * 75.0,
                        Timeframe::M30 => 60.0 * 150.0,
                        Timeframe::H1 => 60.0 * 60.0 * 5.0,
                        Timeframe::H4 => 60.0 * 60.0 * 20.0,
                        _ => 60.0 * 60.0 * 24.0 * 5.0,
                    };
                    self.view_window_start = self.latest_timestamp + buffer - self.window_size;
                    self.is_live_mode = true;
                }
                
                ui.separator();
                
                if self.is_loading {
                    ui.colored_label(egui::Color32::YELLOW, "로딩중...");
                } else {
                    if self.is_live_mode {
                        ui.colored_label(egui::Color32::GREEN, "🔴 라이브");
                    } else {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "📜 히스토리");
                    }
                    ui.separator();
                    
                    let window_end = self.view_window_start + self.window_size;
                    let is_at_edge = (window_end - self.latest_timestamp).abs() < 60.0;
                    ui.label(format!("윈도우: {} ~ {}", 
                        chrono::DateTime::from_timestamp(self.view_window_start as i64, 0)
                            .map(|dt| dt.format("%H:%M").to_string())
                            .unwrap_or("--:--".to_string()),
                        chrono::DateTime::from_timestamp(window_end as i64, 0)
                            .map(|dt| dt.format("%H:%M").to_string())
                            .unwrap_or("--:--".to_string())
                    ));
                    ui.label(format!("최신 데이터: {}", if is_at_edge { "✅" } else { "❌" }));
                    let data_count = if let Ok(data) = self.candle_data.lock() { data.len() } else { 0 };
                    ui.label(format!("데이터: {}개", data_count));
                }
            });
        });
        
        // Main chart area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("BTC/USDT 차트 ({})", self.timeframe.to_display_string()));
            
            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "데이터를 가져오는 중...");
                });
                return;
            }
            
            let data = self.candle_data.lock().unwrap();
            if data.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, "데이터가 없습니다");
                });
                return;
            }
            
            let mut view_window_start = self.view_window_start;
            let window_size = self.window_size;
            let latest_timestamp = self.latest_timestamp;
            let chart_type = self.chart_type.clone();
            let candle_width = self.candle_width;
            
            let plot = Plot::new("crypto_chart")
                .view_aspect(2.0)
                .allow_zoom([false, false])
                .allow_drag([true, false])
                .allow_scroll(false)
                .auto_bounds(egui::Vec2b::new(false, true))
                .default_x_bounds(view_window_start, view_window_start + window_size);
            
            let plot_response = plot.show(ui, |plot_ui| {
                if plot_ui.response().dragged() {
                    let drag_delta = plot_ui.pointer_coordinate_drag_delta();
                    if drag_delta.x.abs() > 0.1 {
                        let proposed_start = view_window_start - drag_delta.x as f64;
                        let proposed_end = proposed_start + window_size;
                        
                        if proposed_end <= latest_timestamp && proposed_start >= 0.0 {
                            view_window_start = proposed_start;
                            self.is_live_mode = false;
                        } else if proposed_end > latest_timestamp {
                            view_window_start = latest_timestamp - window_size;
                            self.is_live_mode = true;
                        } else if proposed_start < 0.0 {
                            view_window_start = 0.0;
                            self.is_live_mode = false;
                        }
                    }
                }
                
                let window_end = view_window_start + window_size;
                let filtered_data: Vec<_> = data
                    .iter()
                    .filter(|candle| {
                        let margin = window_size * 0.1;
                        candle.timestamp >= (view_window_start - margin) && 
                        candle.timestamp <= (window_end + margin)
                    })
                    .cloned()
                    .collect();
                
                match chart_type {
                    ChartType::Line => {
                        let price_points: PlotPoints = filtered_data
                            .iter()
                            .map(|candle| [candle.timestamp, candle.close])
                            .collect();
                        
                        let price_line = Line::new("종가", price_points)
                            .color(egui::Color32::from_rgb(100, 200, 255))
                            .width(2.0);
                        
                        plot_ui.line(price_line);
                    },
                    ChartType::Candlestick => {
                        let mut box_elements = Vec::new();
                        
                        for candle in &filtered_data {
                            let is_bullish = candle.close >= candle.open;
                            let color = if is_bullish {
                                egui::Color32::from_rgb(0, 255, 150)
                            } else {
                                egui::Color32::from_rgb(255, 80, 80)
                            };
                            
                            let box_spread = BoxSpread::new(
                                candle.low,
                                candle.open.min(candle.close),
                                (candle.open + candle.close) / 2.0,
                                candle.open.max(candle.close),
                                candle.high,
                            );
                            
                            let box_elem = BoxElem::new(candle.timestamp, box_spread)
                                .whisker_width(0.1)
                                .box_width(candle_width)
                                .fill(color)
                                .stroke(egui::Stroke::new(1.0, color));
                            
                            box_elements.push(box_elem);
                        }
                        
                        let candlestick_plot = BoxPlot::new("캔들스틱", box_elements);
                        plot_ui.box_plot(candlestick_plot);
                    }
                }
            });
            
            self.is_dragging = plot_response.response.dragged();
            
            if plot_response.response.drag_stopped() {
                let window_end = self.view_window_start + self.window_size;
                let margin = self.window_size * 0.1;
                let fetch_start = self.view_window_start - margin;
                let fetch_end = window_end + margin;
                
                if let Some(rt) = &self.runtime {
                    let candle_data_clone = self.candle_data.clone();
                    let timeframe_clone = self.timeframe.clone();
                    
                    rt.spawn(async move {
                        match fetch_klines_for_range(fetch_start, fetch_end, &timeframe_clone).await {
                            Ok(new_candles) => {
                                if let Ok(mut data) = candle_data_clone.lock() {
                                    for new_candle in new_candles {
                                        if !data.iter().any(|existing| 
                                            (existing.timestamp - new_candle.timestamp).abs() < 1.0) {
                                            data.push_back(new_candle);
                                        }
                                    }
                                    
                                    let mut sorted_data: Vec<_> = data.drain(..).collect();
                                    sorted_data.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
                                    data.extend(sorted_data);
                                }
                            }
                            Err(e) => {
                                eprintln!("데이터 가져오기 실패: {}", e);
                            }
                        }
                    });
                }
            }
            
            self.view_window_start = view_window_start;
        });
        
        // Repaint every second
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}