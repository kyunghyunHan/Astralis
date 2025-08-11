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

struct CryptoApp {
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
    chart_type: ChartType,
    candle_width: f64,
    is_loading: bool,
    runtime: Option<tokio::runtime::Runtime>,
    data_receiver: Option<mpsc::UnboundedReceiver<Vec<CandleData>>>,
    latest_timestamp: f64,
    view_window_start: f64,
    window_size: f64,
    is_dragging: bool,
    is_live_mode: bool,  // 👈 라이브 모드 추적
}

impl Default for CryptoApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let app = Self {
            candle_data: Arc::new(Mutex::new(VecDeque::new())),
            chart_type: ChartType::Candlestick,
            candle_width: 0.8,
            is_loading: true,
            runtime: Some(tokio::runtime::Runtime::new().unwrap()),
            data_receiver: Some(rx),
            latest_timestamp: 0.0,
            view_window_start: 0.0,
            window_size: 500.0 * 60.0,
            is_dragging: false,
            is_live_mode: true,  // 👈 초기에는 라이브 모드
        };
        
        // Start fetching data
        if let Some(rt) = &app.runtime {
            let candle_data_clone = app.candle_data.clone();
            rt.spawn(fetch_binance_data(tx, candle_data_clone));
        }
        
        app
    }
}

async fn fetch_binance_data(
    tx: mpsc::UnboundedSender<Vec<CandleData>>,
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
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
        match fetch_klines_latest().await {
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
        
        // Wait 5 seconds before next update
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn fetch_klines_latest() -> Result<Vec<CandleData>, Box<dyn std::error::Error>> {
    let url = "https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1m&limit=500";
    
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    
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
    end_time: f64
) -> Result<Vec<CandleData>, Box<dyn std::error::Error>> {
    let start_ms = (start_time * 1000.0) as i64;
    let end_ms = (end_time * 1000.0) as i64;
    
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1m&startTime={}&endTime={}&limit=1000",
        start_ms, end_ms
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
                                let buffer = 60.0 * 5.0; // 5분 버퍼
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
                ui.label("Chart Type:");
                egui::ComboBox::from_id_salt("chart_type")
                    .selected_text(match self.chart_type {
                        ChartType::Line => "Line",
                        ChartType::Candlestick => "Candlestick",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.chart_type, ChartType::Line, "Line");
                        ui.selectable_value(&mut self.chart_type, ChartType::Candlestick, "Candlestick");
                    });
                
                if self.chart_type == ChartType::Candlestick {
                    ui.separator();
                    ui.label("Width:");
                    ui.add(egui::Slider::new(&mut self.candle_width, 0.1..=2.0).text(""));
                }
                
                ui.separator();
                
                if ui.button("Live").clicked() {
                    // 라이브 모드로 전환 - 최신 데이터가 오른쪽에서 여유있게 보이도록
                    let buffer = 60.0 * 5.0; // 5분 버퍼
                    self.view_window_start = self.latest_timestamp + buffer - self.window_size;
                    self.is_live_mode = true;
                }
                
                ui.separator();
                
                if self.is_loading {
                    ui.colored_label(egui::Color32::YELLOW, "Loading...");
                } else {
                    if self.is_live_mode {
                        ui.colored_label(egui::Color32::GREEN, "🔴 LIVE");
                    } else {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "📜 History");
                    }
                    ui.separator();
                    // 더 자세한 디버그 정보
                    let window_end = self.view_window_start + self.window_size;
                    let is_at_edge = (window_end - self.latest_timestamp).abs() < 60.0; // 1분 이내
                    ui.label(format!("Window: {} ~ {}", 
                        chrono::DateTime::from_timestamp(self.view_window_start as i64, 0)
                            .map(|dt| dt.format("%H:%M").to_string())
                            .unwrap_or("--:--".to_string()),
                        chrono::DateTime::from_timestamp(window_end as i64, 0)
                            .map(|dt| dt.format("%H:%M").to_string())
                            .unwrap_or("--:--".to_string())
                    ));
                    ui.label(format!("At edge: {}", if is_at_edge { "✅ Yes" } else { "❌ No" }));
                    let data_count = if let Ok(data) = self.candle_data.lock() { data.len() } else { 0 };
                    ui.label(format!("Data: {} candles", data_count));
                }
            });
        });
        
        // Main chart area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BTC/USDT Chart");
            
            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "Loading data...");
                });
                return;
            }
            
            let data = self.candle_data.lock().unwrap();
            if data.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, "No data available");
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
                .auto_bounds(egui::Vec2b::new(false, true))  // X축 자동 바운드 비활성화, Y축 활성화
                .default_x_bounds(view_window_start, view_window_start + window_size);
            
            let plot_response = plot.show(ui, |plot_ui| {
                // 일반적인 드래그 처리 - bounds가 알아서 제한함
                if plot_ui.response().dragged() {
                    let drag_delta = plot_ui.pointer_coordinate_drag_delta();
                    if drag_delta.x.abs() > 0.1 {
                        let proposed_start = view_window_start - drag_delta.x as f64;
                        let proposed_end = proposed_start + window_size;
                        
                        // 범위 제한
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
                
                // Filter data for current window - 여유분 추가
                let window_end = view_window_start + window_size;
                let filtered_data: Vec<_> = data
                    .iter()
                    .filter(|candle| {
                        // 윈도우보다 약간 더 넓은 범위로 필터링 (여유분 10%)
                        let margin = window_size * 0.1;
                        candle.timestamp >= (view_window_start - margin) && 
                        candle.timestamp <= (window_end + margin)
                    })
                    .cloned()
                    .collect();
                
                println!("🔍 Window: {:.2} ~ {:.2}, Filtered: {} candles", 
                    view_window_start, window_end, filtered_data.len());
                
                match chart_type {
                    ChartType::Line => {
                        let price_points: PlotPoints = filtered_data
                            .iter()
                            .map(|candle| [candle.timestamp, candle.close])
                            .collect();
                        
                        let price_line = Line::new("Closing Price", price_points)
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
                        
                        let candlestick_plot = BoxPlot::new("candlestick", box_elements);
                        plot_ui.box_plot(candlestick_plot);
                    }
                }
            });
            
            // Update dragging state
            self.is_dragging = plot_response.response.dragged();
            
            // 드래그가 끝났을 때 현재 차트 범위의 데이터 가져오기
            if plot_response.response.drag_stopped() {
                let window_end = self.view_window_start + self.window_size;
                let margin = self.window_size * 0.1; // 10% 여유분
                let fetch_start = self.view_window_start - margin;
                let fetch_end = window_end + margin;
                
                println!("🎯 차트 범위 변경! 데이터 요청: {:.2} ~ {:.2}", fetch_start, fetch_end);
                
                if let Some(rt) = &self.runtime {
                    let candle_data_clone = self.candle_data.clone();
                    
                    rt.spawn(async move {
                        match fetch_klines_for_range(fetch_start, fetch_end).await {
                            Ok(new_candles) => {
                                if let Ok(mut data) = candle_data_clone.lock() {
                                    // 중복 제거하면서 병합
                                    for new_candle in new_candles {
                                        if !data.iter().any(|existing| 
                                            (existing.timestamp - new_candle.timestamp).abs() < 1.0) {
                                            data.push_back(new_candle);
                                        }
                                    }
                                    
                                    // 타임스탬프 순으로 정렬
                                    let mut sorted_data: Vec<_> = data.drain(..).collect();
                                    sorted_data.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
                                    data.extend(sorted_data);
                                }
                                println!("✅ 차트 범위 데이터 업데이트 완료!");
                            }
                            Err(e) => {
                                println!("❌ 차트 범위 데이터 가져오기 실패: {}", e);
                            }
                        }
                    });
                }
            }
            
            // Update view window
            self.view_window_start = view_window_start;
        });
        
        // Repaint every second
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}