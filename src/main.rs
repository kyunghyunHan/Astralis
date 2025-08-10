#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, VLine, HLine, BoxPlot, BoxElem, BoxSpread};
use std::collections::VecDeque;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("🚀 Cosmic Trader - Galactic Stock Exchange Simulator")
            .with_decorations(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "Cosmic Trader",
        options,
        Box::new(|cc| {
            // Set dark theme for space feel
            cc.egui_ctx.set_visuals(egui::Visuals {
                dark_mode: true,
                override_text_color: Some(egui::Color32::from_rgb(200, 220, 255)),
                window_fill: egui::Color32::from_rgb(15, 20, 35),
                panel_fill: egui::Color32::from_rgb(20, 25, 40),
                ..egui::Visuals::dark()
            });
            
            Ok(Box::<TradingApp>::default())
        }),
    )
}

#[derive(Clone)]
struct StockData {
    timestamp: f64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Clone)]
struct Trade {
    timestamp: f64,
    price: f64,
    quantity: i32,
    trade_type: TradeType,
}

#[derive(Clone, PartialEq)]
enum TradeType {
    Buy,
    Sell,
}

#[derive(Clone, PartialEq)]
enum ChartType {
    Line,
    Candlestick,
}

struct Portfolio {
    credits: f64,  // Changed from cash to credits for space theme
    shares: i32,   // Changed from stocks to shares
    avg_buy_price: f64,
}

impl Portfolio {
    fn total_value(&self, current_price: f64) -> f64 {
        self.credits + (self.shares as f64 * current_price)
    }
    
    fn unrealized_pnl(&self, current_price: f64) -> f64 {
        if self.shares > 0 {
            (current_price - self.avg_buy_price) * self.shares as f64
        } else {
            0.0
        }
    }
}

struct TradingApp {
    stock_data: VecDeque<StockData>,
    trades: Vec<Trade>,
    portfolio: Portfolio,
    current_time: f64,
    is_running: bool,
    speed: f64,
    trade_quantity: i32,
    selected_stock: String,
    show_trades: bool,
    show_volume: bool,
    chart_type: ChartType,
    candle_width: f64,
}

impl Default for TradingApp {
    fn default() -> Self {
        let mut app = Self {
            stock_data: VecDeque::new(),
            trades: Vec::new(),
            portfolio: Portfolio {
                credits: 100000.0,  // Starting with 100k galactic credits
                shares: 0,
                avg_buy_price: 0.0,
            },
            current_time: 0.0,
            is_running: false,
            speed: 1.0,
            trade_quantity: 10,
            selected_stock: "🌟 STARLUX".to_string(),
            show_trades: true,
            show_volume: false,
            chart_type: ChartType::Line,
            candle_width: 0.8,
        };
        
        // Generate initial sample data
        app.generate_sample_data();
        app
    }
}

impl TradingApp {
    fn generate_sample_data(&mut self) {
        let mut price: f64 = 150.0;
        let volume_base = 1000.0;
        
        for i in 0..1000 {
            let time = i as f64;
            
            // Generate OHLC data
            let open = price;
            
            // Calculate high, low, close prices
            let volatility = 0.02;
            let high_change = fastrand::f64() * volatility;
            let low_change = -fastrand::f64() * volatility;
            let close_change = (fastrand::f64() - 0.5) * volatility;
            
            let high = open * (1.0 + high_change);
            let low = open * (1.0 + low_change);
            let close = open * (1.0 + close_change);
            
            // Ensure logical order (low ≤ open, close ≤ high)
            let high = high.max(open.max(close));
            let low = low.min(open.min(close));
            
            price = close; // Next candle starting point
            price = price.max(10.0).min(500.0);
            
            // Volume simulation
            let volume = volume_base * (1.0 + (fastrand::f64() - 0.5) * 0.5);
            
            self.stock_data.push_back(StockData {
                timestamp: time,
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }
    
    fn execute_buy(&mut self, price: f64, quantity: i32) {
        let cost = price * quantity as f64;
        if self.portfolio.credits >= cost {
            // Calculate average buy price
            let total_cost = (self.portfolio.avg_buy_price * self.portfolio.shares as f64) + cost;
            let total_shares = self.portfolio.shares + quantity;
            
            self.portfolio.avg_buy_price = if total_shares > 0 {
                total_cost / total_shares as f64
            } else {
                0.0
            };
            
            self.portfolio.credits -= cost;
            self.portfolio.shares += quantity;
            
            self.trades.push(Trade {
                timestamp: self.current_time,
                price,
                quantity,
                trade_type: TradeType::Buy,
            });
        }
    }
    
    fn execute_sell(&mut self, price: f64, quantity: i32) {
        if self.portfolio.shares >= quantity {
            let revenue = price * quantity as f64;
            self.portfolio.credits += revenue;
            self.portfolio.shares -= quantity;
            
            // Reset average buy price if all shares are sold
            if self.portfolio.shares == 0 {
                self.portfolio.avg_buy_price = 0.0;
            }
            
            self.trades.push(Trade {
                timestamp: self.current_time,
                price,
                quantity,
                trade_type: TradeType::Sell,
            });
        }
    }
    
    fn get_current_price(&self) -> f64 {
        if let Some(data) = self.stock_data.back() {
            data.close
        } else {
            0.0
        }
    }
    
    fn update_simulation(&mut self) {
        if self.is_running {
            self.current_time += self.speed;
            
            // Generate new data point
            if let Some(last_data) = self.stock_data.back() {
                let open = last_data.close;
                let volatility = 0.01;
                
                let high_change = fastrand::f64() * volatility;
                let low_change = -fastrand::f64() * volatility;
                let close_change = (fastrand::f64() - 0.5) * volatility;
                
                let high = open * (1.0 + high_change);
                let low = open * (1.0 + low_change);
                let close = open * (1.0 + close_change);
                
                // Ensure logical order
                let high = high.max(open.max(close));
                let low = low.min(open.min(close));
                let close = close.max(10.0).min(500.0);
                
                let volume = 1000.0 * (1.0 + (fastrand::f64() - 0.5) * 0.5);
                
                self.stock_data.push_back(StockData {
                    timestamp: self.current_time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
                
                // Remove old data (keep only recent 500)
                if self.stock_data.len() > 500 {
                    self.stock_data.pop_front();
                }
            }
        }
    }
    
    fn create_candlestick_boxplot(&self) -> BoxPlot {
        let mut box_elements = Vec::new();
        
        for data in &self.stock_data {
            let is_bullish = data.close >= data.open;
            let color = if is_bullish {
                egui::Color32::from_rgb(0, 255, 150) // Bullish: Neon green
            } else {
                egui::Color32::from_rgb(255, 80, 80) // Bearish: Neon red
            };
            
            let box_spread = BoxSpread::new(
                data.low,           // minimum (low)
                data.open.min(data.close),  // first quartile (lower of open/close)
                (data.open + data.close) / 2.0, // median (middle of open/close)
                data.open.max(data.close),  // third quartile (higher of open/close)
                data.high           // maximum (high)
            );
            
            let box_elem = BoxElem::new(data.timestamp, box_spread)
                .whisker_width(0.1)
                .box_width(self.candle_width)
                .fill(color)
                .stroke(egui::Stroke::new(1.0, color));
            
            box_elements.push(box_elem);
        }
        
        BoxPlot::new("candlestick", box_elements)
    }
}

impl eframe::App for TradingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update simulation
        self.update_simulation();
        
        // Top panel - Controls
        egui::TopBottomPanel::top("control_panel").show(ctx, |ui| {
            // Space-themed styling
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 35, 50);
            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 45, 65);
            ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 60, 80);
            
            ui.horizontal(|ui| {
                ui.label("🌌 Galactic Stock:");
                egui::ComboBox::from_id_salt("stock_selector")
                    .selected_text(&self.selected_stock)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_stock, "🌟 STARLUX".to_string(), "🌟 STARLUX - Stellar Luxuries Corp");
                        ui.selectable_value(&mut self.selected_stock, "🚀 ROCKETECH".to_string(), "🚀 ROCKETECH - Rocket Technologies");
                        ui.selectable_value(&mut self.selected_stock, "🛸 ALIENWARE".to_string(), "🛸 ALIENWARE - Alien Technologies");
                        ui.selectable_value(&mut self.selected_stock, "⭐ NEBULA".to_string(), "⭐ NEBULA - Nebula Mining Co.");
                        ui.selectable_value(&mut self.selected_stock, "🌙 MOONBASE".to_string(), "🌙 MOONBASE - Lunar Habitats");
                        ui.selectable_value(&mut self.selected_stock, "🪐 SATURN".to_string(), "🪐 SATURN - Ring Mining Corp");
                    });
                
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
                
                let button_text = if self.is_running { "⏸️ Pause Warp" } else { "🚀 Engage Warp" };
                if ui.button(button_text).clicked() {
                    self.is_running = !self.is_running;
                }
                
                ui.label("🌊 Warp Speed:");
                ui.add(egui::Slider::new(&mut self.speed, 0.1..=5.0).text("x"));
                
                ui.separator();
                
                ui.checkbox(&mut self.show_trades, "📡 Show Transactions");
                ui.checkbox(&mut self.show_volume, "📈 Show Quantum Volume");
                
                if self.chart_type == ChartType::Candlestick {
                    ui.label("🕯️ Flare Width:");
                    ui.add(egui::Slider::new(&mut self.candle_width, 0.1..=2.0).text(""));
                }
            });
        });
        
        // Bottom panel - Portfolio info
        egui::TopBottomPanel::bottom("portfolio_panel").show(ctx, |ui| {
            let current_price = self.get_current_price();
            let total_value = self.portfolio.total_value(current_price);
            let unrealized_pnl = self.portfolio.unrealized_pnl(current_price);
            
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("🏦 Galactic Portfolio");
                        ui.label(format!("💰 Credits: ¢{:.2}", self.portfolio.credits));
                        ui.label(format!("📊 Shares: {} units", self.portfolio.shares));
                        ui.label(format!("📈 Avg Buy Price: ¢{:.2}", self.portfolio.avg_buy_price));
                        ui.label(format!("💎 Total Assets: ¢{:.2}", total_value));
                        
                        let pnl_text = format!("🎯 Unrealized P&L: ¢{:.2}", unrealized_pnl);
                        let pnl_color = if unrealized_pnl >= 0.0 {
                            egui::Color32::from_rgb(0, 255, 150)
                        } else {
                            egui::Color32::from_rgb(255, 80, 80)
                        };
                        ui.colored_label(pnl_color, pnl_text);
                    });
                });
                
                ui.separator();
                
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("⚡ Trading Station");
                        ui.label(format!("🎯 Current Price: ¢{:.2}", current_price));
                        ui.horizontal(|ui| {
                            ui.label("📦 Quantity:");
                            ui.add(egui::DragValue::new(&mut self.trade_quantity).range(1..=1000));
                        });
                        
                        ui.horizontal(|ui| {
                            let buy_cost = current_price * self.trade_quantity as f64;
                            let can_buy = self.portfolio.credits >= buy_cost;
                            let can_sell = self.portfolio.shares >= self.trade_quantity;
                            
                            let buy_button = egui::Button::new(format!("🟢 BUY (¢{:.0})", buy_cost))
                                .fill(egui::Color32::from_rgb(0, 100, 50));
                            if ui.add_enabled(can_buy, buy_button).clicked() {
                                self.execute_buy(current_price, self.trade_quantity);
                            }
                            
                            let sell_button = egui::Button::new(format!("🔴 SELL (¢{:.0})", current_price * self.trade_quantity as f64))
                                .fill(egui::Color32::from_rgb(100, 30, 30));
                            if ui.add_enabled(can_sell, sell_button).clicked() {
                                self.execute_sell(current_price, self.trade_quantity);
                            }
                        });
                    });
                });
                
                ui.separator();
                
                // Recent transaction history
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("📡 Recent Transmissions");
                        egui::ScrollArea::vertical()
                            .max_height(80.0)
                            .show(ui, |ui| {
                                for trade in self.trades.iter().rev().take(5) {
                                    let (trade_type_str, icon, color) = match trade.trade_type {
                                        TradeType::Buy => ("BUY", "🟢", egui::Color32::from_rgb(0, 255, 150)),
                                        TradeType::Sell => ("SELL", "🔴", egui::Color32::from_rgb(255, 80, 80)),
                                    };
                                    ui.colored_label(color, format!("{} {} {} units @ ¢{:.2}", 
                                        icon, trade_type_str, trade.quantity, trade.price));
                                }
                            });
                    });
                });
            });
        });
        
        // Left panel - Chart
        egui::SidePanel::left("chart_panel").min_width(800.0).show(ctx, |ui| {
            ui.heading(format!("🌌 {} Galactic Exchange ({})", 
                self.selected_stock,
                match self.chart_type {
                    ChartType::Line => "Plasma Line",
                    ChartType::Candlestick => "Solar Flares",
                }
            ));
            
            let plot = Plot::new("stock_chart")
                .view_aspect(2.0)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_scroll(true)
                .show_axes([true, true])
                .show_grid([true, true]);
            
            plot.show(ui, |plot_ui| {
                match self.chart_type {
                    ChartType::Line => {
                        // Line chart (closing price based)
                        let price_points: PlotPoints = self.stock_data
                            .iter()
                            .map(|data| [data.timestamp, data.close])
                            .collect();
                        
                        let price_line = Line::new("Closing Price", price_points)
                            .color(egui::Color32::from_rgb(100, 200, 255))
                            .width(2.0);
                        
                        plot_ui.line(price_line);
                    },
                    ChartType::Candlestick => {
                        // Candlestick chart (using BoxPlot)
                        let candlestick_plot = self.create_candlestick_boxplot();
                        plot_ui.box_plot(candlestick_plot);
                    }
                }
                
                // Volume display (secondary axis)
                if self.show_volume {
                    let volume_points: PlotPoints = self.stock_data
                        .iter()
                        .map(|data| [data.timestamp, data.volume / 10.0]) // Scale adjustment
                        .collect();
                    
                    let volume_line = Line::new("Quantum Volume (÷10)", volume_points)
                        .color(egui::Color32::from_rgb(150, 150, 200))
                        .width(1.0);
                    
                    plot_ui.line(volume_line);
                }
                
                // Trade markers
                if self.show_trades {
                    for trade in &self.trades {
                        let color = match trade.trade_type {
                            TradeType::Buy => egui::Color32::from_rgb(0, 255, 150),
                            TradeType::Sell => egui::Color32::from_rgb(255, 80, 80),
                        };
                        
                        // Vertical line for trade timing
                        plot_ui.vline(
                            VLine::new("Trade Time", trade.timestamp)
                                .color(color)
                                .width(2.0)
                        );
                        
                        // Horizontal line for trade price
                        plot_ui.hline(
                            HLine::new("Trade Price", trade.price)
                                .color(color)
                                .width(1.0)
                        );
                    }
                }
                
                // Current price display
                if let Some(current_data) = self.stock_data.back() {
                    plot_ui.hline(
                        HLine::new("Current Price", current_data.close)
                            .color(egui::Color32::from_rgb(255, 255, 0))
                            .width(3.0)
                    );
                }
            });
        });
        
        // Right panel - Transaction history and statistics
        egui::SidePanel::right("info_panel").min_width(300.0).show(ctx, |ui| {
            ui.heading("📜 Transaction Log");
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("trade_history")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("⏰ Time");
                        ui.strong("🎯 Type");
                        ui.strong("📦 Qty");
                        ui.strong("💰 Price");
                        ui.strong("💎 Total");
                        ui.end_row();
                        
                        for trade in self.trades.iter().rev() {
                            ui.label(format!("{:.0}", trade.timestamp));
                            
                            let (type_text, color) = match trade.trade_type {
                                TradeType::Buy => ("🟢 BUY", egui::Color32::from_rgb(0, 255, 150)),
                                TradeType::Sell => ("🔴 SELL", egui::Color32::from_rgb(255, 80, 80)),
                            };
                            ui.colored_label(color, type_text);
                            
                            ui.label(format!("{}", trade.quantity));
                            ui.label(format!("¢{:.2}", trade.price));
                            ui.label(format!("¢{:.2}", trade.price * trade.quantity as f64));
                            ui.end_row();
                        }
                    });
            });
        });
        
        // 60fps updates
        if self.is_running {
            ctx.request_repaint();
        }
    }
}