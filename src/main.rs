#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use chrono::{DateTime, Local, TimeZone, Datelike, NaiveDateTime};
use eframe::egui;
use egui::{menu, Color32};
use std::io::{Read, Write};

fn load_icon(buffer: &[u8]) -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(buffer)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

const ICON: &[u8] = include_bytes!("../case_notifier.png");

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        icon_data: Some(load_icon(ICON)),
        initial_window_size: Some(egui::vec2(480.0, 560.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Cases Notifier",
        options,
        Box::new(|_cc| Box::<CasesNotifier>::default()),
    )
}

fn format_date(date: u64) -> String {
    let local_date_time = Local.timestamp_opt(date as i64, 0).unwrap();
    local_date_time.format("%H:%M:%S %d/%m/%Y").to_string()
}

fn format_time(time: u64) -> String {
    let seconds = time % 60;
    let minutes = (time / 60) % 60;
    let hours = (time / 3600) % 24;
    let days = time / 86400;

    format!("{}:{:02}:{:02}:{:02}", days, hours, minutes, seconds)
}

struct Account {
    name: String,
    date: u64,
}

impl Account {
    fn new(name: String, date: u64) -> Self {
        Self { name, date }
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_date(&self) -> u64 {
        self.date
    }

    fn get_next_date(&self) -> u64 {
        next_wednesday(self.date)
    }

    fn get_remaining_time(&self) -> u64 {
        let now = chrono::Local::now().timestamp() as u64;
        let next = self.get_next_date();
        if now > next {
            return 0;
        }
        next - now
    }

    fn to_binary(&self) -> Vec<u8> {
        let mut data = vec![];
        data.extend_from_slice(self.name.as_bytes());
        data.push(0);
        data.extend_from_slice(&self.date.to_le_bytes());
        data
    }
}

struct CasesNotifier {
    accounts: Vec<Account>,
    editing_account: bool,
    account_to_edit: usize,
    editing_date: String,
}

impl Default for CasesNotifier {
    fn default() -> Self {
        Self {
            accounts: load_accounts(),
            editing_account: false,
            account_to_edit: 0,
            editing_date: "".to_string(),
        }
    }
}

fn save_accounts(accounts: &Vec<Account>) {
    let mut file = std::fs::File::create("accounts.dat").unwrap();
    for account in accounts {
        file.write_all(&account.to_binary()).unwrap();
    }
}

fn load_accounts() -> Vec<Account> {
    let mut accounts = vec![];
    if let Ok(file) = std::fs::File::open("accounts.dat") {
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = vec![];
        loop {
            let mut byte = [0; 1];
            if reader.read(&mut byte).unwrap() == 0 {
                break;
            }
            if byte[0] == 0 {
                let name = String::from_utf8(buffer.clone()).unwrap();
                buffer.clear();
                let mut date = [0; 8];
                reader.read_exact(&mut date).unwrap();
                let date = u64::from_le_bytes(date);
                accounts.push(Account::new(name, date));
            } else {
                buffer.push(byte[0]);
            }
        }
    }
    accounts
}

fn next_wednesday(timestamp: u64) -> u64 {
    let datetime = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap();

    let mut next_wednesday = datetime.date().succ_opt().unwrap();
    while next_wednesday.weekday() != chrono::Weekday::Wed {
        next_wednesday = next_wednesday.succ_opt().unwrap();
    }

    let next_wednesday_utc = NaiveDateTime::new(next_wednesday, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    next_wednesday_utc.timestamp() as u64
}

impl eframe::App for CasesNotifier {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // menu bar
            menu::bar(ui, |ui| {
                if ui.button("Add account").clicked() && !self.editing_account {
                    self.accounts.push(Account::new(
                        "Account name".to_string(),
                        chrono::Utc::now().timestamp() as u64,
                    ));
                    self.editing_account = true;
                    self.account_to_edit = self.accounts.len() - 1;
                    self.editing_date = format_date(self.accounts[self.account_to_edit].get_date());
                    save_accounts(&self.accounts);
                }

                let mut count = 0;
                for account in &self.accounts {
                    if account.get_remaining_time() <= 0 {
                        count += 1;
                    }
                }
                ui.label(format!("Accounts ready: {}/{}", count, self.accounts.len()));
            });

            // scrollable area
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut to_delete_index = -1;
                let mut to_save = false;

                for (i, account) in &mut self.accounts.iter_mut().enumerate() {
                    ui.label(egui::RichText::new(account.get_name()).strong().size(18.0));
                    ui.label(format!("Last drop: {}", format_date(account.get_date())));
                    ui.label(format!(
                        "Next drop: {}",
                        format_date(account.get_next_date())
                    ));
                    let remaining_time = account.get_remaining_time();
                    if remaining_time > 0 {
                        ui.colored_label(Color32::from_rgb(255, 50, 75), format!("Remaining: {}", format_time(remaining_time)));
                    } else {
                        ui.colored_label(Color32::from_rgb(50, 255, 75), "Ready!");
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Edit").clicked() && !self.editing_account {
                            self.editing_account = true;
                            self.account_to_edit = i;
                            self.editing_date = format_date(account.get_date());
                        }

                        if ui.button("Delete").clicked() && !self.editing_account {
                            to_delete_index = i as i32;
                        }

                        if ui.button("Reset timer").clicked() && !self.editing_account {
                            account.date = chrono::Utc::now().timestamp() as u64;
                            to_save = true;
                        }
                    });

                    ui.separator();
                }

                if to_delete_index != -1 {
                    self.accounts.remove(to_delete_index as usize);
                    to_save = true;
                }

                if to_save {
                    save_accounts(&self.accounts);
                }
            });

            // edit account
            if self.editing_account {
                let mut show_window = true;
                egui::Window::new("Edit account")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .resizable(false)
                    .collapsible(false)
                    .open(&mut show_window)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Account name:");
                            ui.add(egui::TextEdit::singleline(
                                &mut self.accounts[self.account_to_edit].name,
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Last drop:");
                            ui.add(egui::TextEdit::singleline(&mut self.editing_date));
                        });

                        // try to parse date
                        match chrono::NaiveDateTime::parse_from_str(
                            &self.editing_date,
                            "%H:%M:%S %d/%m/%Y",
                        ) {
                            Ok(date) => {
                                // convert datetime from local to utc
                                let local_date_time: DateTime<Local> =
                                    Local.from_local_datetime(&date).unwrap();
                                self.accounts[self.account_to_edit].date =
                                    local_date_time.timestamp() as u64;
                            }
                            Err(_) => {}
                        }
                    });

                if !show_window {
                    self.editing_account = false;
                    save_accounts(&self.accounts);
                }
            }

            // update window every second
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        });
    }
}
