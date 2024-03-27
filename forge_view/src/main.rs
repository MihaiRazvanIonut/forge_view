use clap::Parser;
use eframe::egui::{self, Vec2, Visuals};
use egui_extras::{Column, TableBuilder};
use process::{build_process_tree, Process, ProcessTree, ProcessTreeNode, System};

#[derive(Parser)]
#[command(version, about = "Forge View launch commands")]
struct Args {
    #[arg(short, long, default_value = None)]
    width: Option<f32>,
    #[arg(short, long, default_value = None)]
    lheigth: Option<f32>,
}

const F32_PRECISION: usize = 2;

fn main() -> Result<(), eframe::Error> {
    let args = Args::parse();
    let mut native_options = eframe::NativeOptions::default();
    if let (Some(width), Some(heigth)) = (args.width, args.lheigth) {
        if width == 0f32 || heigth == 0f32 {
            println!("Error: Window width or heigth size can not be 0!");
            println!("Using default window sizes");
        } else {
            native_options.viewport.inner_size = Option::from(Vec2::new(width, heigth));
        }
    }
    eframe::run_native(
        "Forge View",
        native_options,
        Box::new(|cc| Box::new(ForgeViewApp::new(cc))),
    )
}
enum AppStates {
    ProcList,
    ProcTree,
}

struct ForgeViewApp {
    metric_state: AppStates,
    system_metric: System,
    system_list: Vec<(u32, Process)>,
    system_tree: ProcessTree,
    dark_mode: bool,
}

impl Default for ForgeViewApp {
    fn default() -> Self {
        let mut system = System::new();
        match system.refresh_system_info() {
            Ok(_) => {}
            Err(_) => println!("Error: Process lib could not compute metrics!"),
        }
        let process_tree = process::build_process_tree(&system);
        let sys_vector = system.get_procs_as_list();
        Self {
            dark_mode: true,
            metric_state: AppStates::ProcList,
            system_metric: system,
            system_list: sys_vector,
            system_tree: process_tree,
        }
    }
}

impl ForgeViewApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_fonts(egui::FontDefinitions::default());
        Self::default()
    }
}

impl eframe::App for ForgeViewApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(sys_theme) = frame.info().system_theme {
            match sys_theme {
                eframe::Theme::Dark => ctx.set_visuals(Visuals::dark()),
                eframe::Theme::Light => ctx.set_visuals(Visuals::light()),
            }
        }
        egui::TopBottomPanel::top("Metrics Buttons").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.button("Process List").clicked() {
                        self.metric_state = AppStates::ProcList;
                        match self.system_metric.refresh_system_info() {
                            Ok(_) => {}
                            Err(_) => println!("Error: Process lib could not compute metrics!"),
                        }
                        self.system_list = self.system_metric.get_procs_as_list();
                        ui.ctx().request_repaint();
                    }
                    if ui.button("Process Tree").clicked() {
                        self.metric_state = AppStates::ProcTree;
                        match self.system_metric.refresh_system_info() {
                            Ok(_) => {}
                            Err(_) => println!("Error: Process lib could not compute metrics!"),
                        }
                        self.system_tree = build_process_tree(&self.system_metric);
                        ui.ctx().request_repaint();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🌙").clicked() {
                        self.dark_mode = !self.dark_mode;
                        match self.dark_mode {
                            true => ctx.set_visuals(Visuals::dark()),
                            false => ctx.set_visuals(Visuals::light()),
                        }
                    }
                    if ui.button("⟳").clicked() {
                        match self.system_metric.refresh_system_info() {
                            Ok(_) => {}
                            Err(_) => println!("Error: Process lib could not compute metrics!"),
                        }
                        match self.metric_state {
                            AppStates::ProcList => {
                                self.system_list = self.system_metric.get_procs_as_list();
                            }
                            AppStates::ProcTree => {
                                self.system_tree = build_process_tree(&self.system_metric);
                            }
                        }
                        ui.ctx().request_repaint();
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("System Usage").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("System Usage:");
                ui.label(format!(
                    "CPU usage: %{:.1$}",
                    self.system_metric.get_total_cpu_usage(),
                    F32_PRECISION
                ));
                ui.label(format!(
                    "Memory usage: %{:.1$}",
                    self.system_metric.get_total_mem_usage(),
                    F32_PRECISION
                ));
            });
        });
        match self.metric_state {
            AppStates::ProcList => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::remainder().clip(true).resizable(true))
                        .column(Column::remainder().clip(true).resizable(true))
                        .column(Column::remainder().clip(true).resizable(true))
                        .column(Column::remainder().clip(true).resizable(true))
                        .column(Column::remainder().clip(true).resizable(true))
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.heading("Name");
                            });
                            header.col(|ui| {
                                ui.heading("%CPU");
                            });
                            header.col(|ui| {
                                ui.heading("%MEM");
                            });
                            header.col(|ui| {
                                ui.heading("Path");
                            });
                            header.col(|ui| {
                                ui.heading("User");
                            });
                        })
                        .body(|body| {
                            body.rows(20.0, self.system_list.len(), |mut row| {
                                let row_index = row.index();
                                row.col(|ui| {
                                    ui.label(self.system_list[row_index].1.get_name());
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{:.1$}",
                                        self.system_list[row_index].1.get_cpu_used(),
                                        F32_PRECISION
                                    ));
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{:.1$}",
                                        self.system_list[row_index].1.get_mem_used(),
                                        F32_PRECISION
                                    ));
                                });
                                row.col(|ui| {
                                    ui.label(self.system_list[row_index].1.get_path());
                                });
                                row.col(|ui| {
                                    ui.label(self.system_list[row_index].1.get_user());
                                });
                            });
                        });
                });
            }
            AppStates::ProcTree => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::new([false, true])
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            tree_layout(ui, &self.system_tree.root);
                        });
                });
            }
        }
    }
}

fn tree_layout(ui: &mut egui::Ui, proc_node: &ProcessTreeNode) {
    egui::CollapsingHeader::new(format!(
        "{} - PID: {}",
        proc_node.proc_info.get_name(),
        proc_node.proc_info.get_pid()
    ))
    .default_open(true)
    .show(ui, |ui| {
        for child in proc_node.children.iter() {
            tree_layout(ui, child);
        }
    });
}
