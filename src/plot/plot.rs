use eframe::egui::{self, Color32, Stroke, Vec2};
use egui_plot::{Bar, BarChart, BoxElem, BoxPlot, BoxSpread, Line, Orientation, Plot, PlotPoints};
use reqwest::{header, Error};
use std::time::{Duration, UNIX_EPOCH};
use yahoo_finance_api::time::{macros::datetime, OffsetDateTime};

pub fn bar_chart(ui: &mut egui::Ui, bars: &Vec<BoxElem>) {
    // let bar_data= BarChart::new(bars.iter().map(|(bar, _)| bar.clone()).collect());
    // let box_plot= BoxPlot::new(bars.iter().map(|(_, plot)| plot.clone()).collect());
    let box_plot = BoxPlot::new(bars.clone());
    let plot = Plot::new("candlestick chart")
        .view_aspect(3.0)
        .min_size(Vec2::from([12., 12.]));

    plot.show(ui, |plot_ui| {
        // plot_ui.bar_chart(bar_data);
        plot_ui.box_plot(box_plot);
    });
}
