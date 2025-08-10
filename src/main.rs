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
            .with_title("🚀 Cosmic Crypto Viewer - Live Binance Charts"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Cosmic Crypto Viewer",
        options,
        Box::new(|cc| {
            // Set dark space theme
            cc.egui_ctx.set_visuals(egui::Visuals {
                dark_mode: true,
                override_text_color: Some(egui::Color32::from_rgb(200, 220, 255)),
                window_fill: egui::Color32::from_rgb(15, 20, 35),
                panel_fill: egui::Color32::from_rgb(20, 25, 40),
                ..egui::Visuals::dark()
            });
            
            Ok(Box::<CryptoApp>::default())
        }),
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
enum CryptoSymbol {
    Bitcoin,
    Ethereum,
}

impl CryptoSymbol {
    fn to_binance_symbol(&self) -> &str {
        match self {
            CryptoSymbol::Bitcoin => "BTCUSDT",
            CryptoSymbol::Ethereum => "ETHUSDT",
        }
    }
    
    fn display_name(&self) -> &str {
        match self {
            CryptoSymbol::Bitcoin => "₿ Bitcoin",
            CryptoSymbol::Ethereum => "Ξ Ethereum",
        }
    }
    
    fn cosmic_name(&self) -> &str {
        match self {
            CryptoSymbol::Bitcoin => "🟡 Quantum Bitcoin",
            CryptoSymbol::Ethereum => "🔷 Stellar Ethereum",
        }
    }
}

struct CryptoApp {
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
    selected_crypto: CryptoSymbol,
    chart_type: ChartType,
    candle_width: f64,
    current_price: f64,
    price_change_24h: f64,
    is_loading: bool,
    last_update: std::time::Instant,
    runtime: Option<tokio::runtime::Runtime>,
    data_receiver: Option<mpsc::UnboundedReceiver<Vec<CandleData>>>,
}

impl Default for CryptoApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let app = Self {
            candle_data: Arc::new(Mutex::new(VecDeque::new())),
            selected_crypto: CryptoSymbol::Bitcoin,
            chart_type: ChartType::Candlestick,
            candle_width: 0.8,
            current_price: 0.0,
            price_change_24h: 0.0,
            is_loading: true,
            last_update: std::time::Instant::now(),
            runtime: Some(tokio::runtime::Runtime::new().unwrap()),
            data_receiver: Some(rx),
        };
        
        // Start fetching data
        if let Some(rt) = &app.runtime {
            let candle_data_clone = app.candle_data.clone();
            rt.spawn(fetch_binance_data(CryptoSymbol::Bitcoin, tx, candle_data_clone));
        }
        
        app
    }
}

async fn fetch_binance_data(
    symbol: CryptoSymbol,
    tx: mpsc::UnboundedSender<Vec<CandleData>>,
    candle_data: Arc<Mutex<VecDeque<CandleData>>>,
) {
    println!("🚀 Starting data fetch for {}", symbol.display_name());
    
    loop {
        match fetch_klines(&symbol).await {
            Ok(candles) => {
                println!("📊 Fetched {} candles for {}", candles.len(), symbol.display_name());
                
                // Update shared data
                if let Ok(mut data) = candle_data.lock() {
                    data.clear();
                    data.extend(candles.iter().cloned());
                }
                
                // Send to UI
                if tx.send(candles).is_err() {
                    println!("❌ Failed to send data to UI");
                    break;
                }
            }
            Err(e) => {
                println!("❌ Error fetching data for {}: {}", symbol.display_name(), e);
            }
        }
        
        // Wait 30 seconds before next update
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

async fn fetch_klines(symbol: &CryptoSymbol) -> Result<Vec<CandleData>, Box<dyn std::error::Error>> {
    let binance_symbol = symbol.to_binance_symbol();
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol={}&interval=1m&limit=500",
        binance_symbol
    );
    
    println!("🌐 Fetching from: {}", url);
    
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
                            timestamp: timestamp / 1000.0, // Convert to seconds
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
    
    println!("✅ Parsed {} valid candles", candles.len());
    Ok(candles)
}

impl CryptoApp {
    fn create_candlestick_boxplot(&self, data: &VecDeque<CandleData>) -> BoxPlot {
        let mut box_elements = Vec::new();
        
        for candle in data.iter() {
            let is_bullish = candle.close >= candle.open;
            let color = if is_bullish {
                egui::Color32::from_rgb(0, 255, 150) // Neon green for bullish
            } else {
                egui::Color32::from_rgb(255, 80, 80) // Neon red for bearish
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
                .box_width(self.candle_width)
                .fill(color)
                .stroke(egui::Stroke::new(1.0, color));
            
            box_elements.push(box_elem);
        }
        
        BoxPlot::new("candlestick", box_elements)
    }
    
    fn switch_crypto(&mut self, new_crypto: CryptoSymbol) {
        // println!("🔄 Switching to: {:?}", new_crypto);
        self.selected_crypto = new_crypto.clone();
        self.is_loading = true;
        
        // Clear existing data
        if let Ok(mut data) = self.candle_data.lock() {
            data.clear();
        }
        
        // Start new data fetch
        if let Some(rt) = &self.runtime {
            let (tx, rx) = mpsc::unbounded_channel();
            self.data_receiver = Some(rx);
            
            let candle_data_clone = self.candle_data.clone();
            rt.spawn(fetch_binance_data(new_crypto, tx, candle_data_clone));
        }
    }
}

impl eframe::App for CryptoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for new data
        if let Some(receiver) = &mut self.data_receiver {
            while let Ok(new_candles) = receiver.try_recv() {
                if !new_candles.is_empty() {
                    self.is_loading = false;
                    self.last_update = std::time::Instant::now();
                    
                    // Update current price and calculate 24h change
                    if let Some(latest) = new_candles.last() {
                        self.current_price = latest.close;
                        
                        // Calculate 24h change (approximate using first candle as 24h ago)
                        if let Some(first) = new_candles.first() {
                            self.price_change_24h = 
                                ((latest.close - first.open) / first.open) * 100.0;
                        }
                    }
                }
            }
        }
        
        // Top panel - Controls
        egui::TopBottomPanel::top("control_panel").show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 35, 50);
            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 45, 65);
            ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 60, 80);
            
            ui.horizontal(|ui| {
                ui.label("🌌 Cosmic Crypto:");
                
                let mut crypto_changed = false;
                let current_crypto = self.selected_crypto.clone();
                
                egui::ComboBox::from_id_salt("crypto_selector")
                    .selected_text(self.selected_crypto.cosmic_name())
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut self.selected_crypto, CryptoSymbol::Bitcoin, "🟡 Quantum Bitcoin").clicked() 
                            && current_crypto != CryptoSymbol::Bitcoin {
                            crypto_changed = true;
                        }
                        if ui.selectable_value(&mut self.selected_crypto, CryptoSymbol::Ethereum, "🔷 Stellar Ethereum").clicked() 
                            && current_crypto != CryptoSymbol::Ethereum {
                            crypto_changed = true;
                        }
                    });
                
                if crypto_changed {
                    // println!("🔄 Crypto selection changed to: {:?}", self.selected_crypto);
                    self.switch_crypto(self.selected_crypto.clone());
                }
                
                ui.separator();
                
                ui.label("📊 Chart Type:");
                egui::ComboBox::from_id_salt("chart_type")
                    .selected_text(match self.chart_type {
                        ChartType::Line => "⚡ Plasma Line",
                        ChartType::Candlestick => "🕯️ Solar Flares",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.chart_type, ChartType::Line, "⚡ Plasma Line");
                        ui.selectable_value(&mut self.chart_type, ChartType::Candlestick, "🕯️ Solar Flares");
                    });
                
                ui.separator();
                
                if self.chart_type == ChartType::Candlestick {
                    ui.label("🕯️ Flare Width:");
                    ui.add(egui::Slider::new(&mut self.candle_width, 0.1..=2.0).text(""));
                }
                
                ui.separator();
                
                // Status indicator
                if self.is_loading {
                    ui.colored_label(egui::Color32::YELLOW, "🔄 Loading cosmic data...");
                } else {
                    let time_since_update = self.last_update.elapsed().as_secs();
                    ui.colored_label(
                        egui::Color32::GREEN, 
                        format!("✅ Live ({}s ago)", time_since_update)
                    );
                }
            });
        });
        
        // Bottom panel - Price info
        egui::TopBottomPanel::bottom("price_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(format!("💰 {}", self.selected_crypto.cosmic_name()));
                        ui.label(format!("🎯 Current Price: ${:.2}", self.current_price));
                        
                        let change_text = format!("📈 24h Change: {:.2}%", self.price_change_24h);
                        let change_color = if self.price_change_24h >= 0.0 {
                            egui::Color32::from_rgb(0, 255, 150)
                        } else {
                            egui::Color32::from_rgb(255, 80, 80)
                        };
                        ui.colored_label(change_color, change_text);
                    });
                });
            });
        });
        
        // Main chart area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("🌌 {} Galactic Exchange ({})", 
                self.selected_crypto.cosmic_name(),
                match self.chart_type {
                    ChartType::Line => "Plasma Line",
                    ChartType::Candlestick => "Solar Flares",
                }
            ));
            
            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "🚀 Connecting to cosmic data streams...");
                });
                return;
            }
            
            let data = self.candle_data.lock().unwrap();
            if data.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, "❌ No cosmic data available");
                });
                return;
            }
            
            let plot = Plot::new("crypto_chart")
                .view_aspect(2.0)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_scroll(true)
                .show_axes([true, true])
                .show_grid([true, true]);
            
            plot.show(ui, |plot_ui| {
                match self.chart_type {
                    ChartType::Line => {
                        let price_points: PlotPoints = data
                            .iter()
                            .map(|candle| [candle.timestamp, candle.close])
                            .collect();
                        
                        let price_line = Line::new("Closing Price", price_points)
                            .color(egui::Color32::from_rgb(100, 200, 255))
                            .width(2.0);
                        
                        plot_ui.line(price_line);
                    },
                    ChartType::Candlestick => {
                        let candlestick_plot = self.create_candlestick_boxplot(&data);
                        plot_ui.box_plot(candlestick_plot);
                    }
                }
            });
        });
        
        // Request repaint every second to update the "time since update"
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}