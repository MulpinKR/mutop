mod proc_table;

use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use sysinfo::System;

use proc_table::ProcInfo;

const ORANGE_LIGHT: Color = Color::Rgb(255, 183, 77);
const ORANGE: Color = Color::Rgb(255, 152, 0);
const ORANGE_DARK: Color = Color::Rgb(230, 81, 0);
const ORANGE_DEEP: Color = Color::Rgb(191, 54, 12);
const BG_DARK: Color = Color::Rgb(30, 25, 20);
const BG_CARD: Color = Color::Rgb(45, 38, 30);
const TEXT: Color = Color::Rgb(255, 243, 224);
const TEXT_DIM: Color = Color::Rgb(180, 160, 140);
const RED: Color = Color::Rgb(229, 80, 80);
const GREEN: Color = Color::Rgb(129, 199, 132);
const CYAN: Color = Color::Rgb(128, 203, 196);

#[derive(PartialEq, Clone, Copy)]
enum SortBy {
    Cpu,
    Mem,
    Pid,
    Name,
}

enum InputMode {
    Normal,
    Searching,
    ConfirmKill,
}

struct App {
    system: System,
    processes: Vec<ProcInfo>,
    table_state: TableState,
    sort_by: SortBy,
    input_mode: InputMode,
    search_query: String,
    search_cursor: usize,
    should_quit: bool,
    auto_refresh: bool,
    selected_pid: u32,
    sort_desc: bool,
    uptime_secs: u64,
}

impl App {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let mut app = Self {
            system,
            processes: Vec::new(),
            table_state: TableState::default(),
            sort_by: SortBy::Cpu,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_cursor: 0,
            should_quit: false,
            auto_refresh: true,
            selected_pid: 0,
            sort_desc: true,
            uptime_secs: 0,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        self.uptime_secs = System::uptime();
        self.processes.clear();

        for (pid, process) in self.system.processes() {
            self.processes.push(ProcInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cmd: process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                status: format!("{:?}", process.status()),
            });
        }

        self.sort();
        self.filter();
    }

    fn sort(&mut self) {
        let desc = self.sort_desc;
        match self.sort_by {
            SortBy::Cpu => {
                self.processes.sort_by(|a, b| {
                    if desc {
                        b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()
                    } else {
                        a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap()
                    }
                });
            }
            SortBy::Mem => {
                self.processes.sort_by(|a, b| {
                    if desc {
                        b.memory.cmp(&a.memory)
                    } else {
                        a.memory.cmp(&b.memory)
                    }
                });
            }
            SortBy::Pid => {
                self.processes.sort_by(|a, b| {
                    if desc {
                        b.pid.cmp(&a.pid)
                    } else {
                        a.pid.cmp(&b.pid)
                    }
                });
            }
            SortBy::Name => {
                self.processes.sort_by(|a, b| {
                    if desc {
                        b.name.cmp(&a.name)
                    } else {
                        a.name.cmp(&b.name)
                    }
                });
            }
        }
    }

    fn filter(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let q = self.search_query.to_lowercase();
        self.processes.retain(|p| {
            p.name.to_lowercase().contains(&q)
                || p.cmd.to_lowercase().contains(&q)
                || p.pid.to_string().contains(&q)
        });
    }

    fn next(&mut self) {
        if self.processes.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.processes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.processes.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.processes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortBy::Cpu => SortBy::Mem,
            SortBy::Mem => SortBy::Pid,
            SortBy::Pid => SortBy::Name,
            SortBy::Name => SortBy::Cpu,
        };
        self.sort_desc = !self.sort_desc;
        self.sort();
    }

    fn start_search(&mut self) {
        self.input_mode = InputMode::Searching;
        self.search_query.clear();
        self.search_cursor = 0;
    }

    fn cancel_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.refresh();
    }

    fn submit_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.refresh();
    }

    fn search_char(&mut self, c: char) {
        self.search_query.insert(self.search_cursor, c);
        self.search_cursor += 1;
    }

    fn search_backspace(&mut self) {
        if self.search_cursor > 0 {
            self.search_query.remove(self.search_cursor - 1);
            self.search_cursor -= 1;
        }
    }

    fn search_left(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
        }
    }

    fn search_right(&mut self) {
        if self.search_cursor < self.search_query.len() {
            self.search_cursor += 1;
        }
    }

    fn confirm_kill(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(proc) = self.processes.get(i) {
                self.selected_pid = proc.pid;
                self.input_mode = InputMode::ConfirmKill;
            }
        }
    }

    fn execute_kill(&mut self) {
        use std::process::Command;
        let pid = self.selected_pid;
        let _ = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output();
        self.input_mode = InputMode::Normal;
        self.refresh();
    }

    fn cancel_kill(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    fn toggle_refresh(&mut self) {
        self.auto_refresh = !self.auto_refresh;
    }

    fn selected_proc_info(&self) -> Option<String> {
        if let Some(i) = self.table_state.selected() {
            if let Some(proc) = self.processes.get(i) {
                return Some(format!(
                    "PID: {}  |  CPU: {:.1}%  |  MEM: {}  |  Status: {}\nCMD: {}",
                    proc.pid,
                    proc.cpu_usage,
                    format_bytes(proc.memory),
                    proc.status,
                    if proc.cmd.is_empty() {
                        &proc.name
                    } else {
                        &proc.cmd
                    }
                ));
            }
        }
        None
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_proc_table(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);

    match app.input_mode {
        InputMode::Searching => render_search_popup(frame, app),
        InputMode::ConfirmKill => render_kill_confirm(frame, app),
        InputMode::Normal => {}
    }
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let sort_label = match app.sort_by {
        SortBy::Cpu => "CPU",
        SortBy::Mem => "MEM",
        SortBy::Pid => "PID",
        SortBy::Name => "NAME",
    };

    let arrow = if app.sort_desc { "▼" } else { "▲" };

    let title = Line::from(vec![
        Span::styled("◆ ", Style::default().fg(ORANGE)),
        Span::styled(
            "MUTOP",
            Style::default()
                .fg(ORANGE_LIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ◆", Style::default().fg(ORANGE)),
    ]);

    let total_procs = app.system.processes().len();
    let total_mem = app.system.total_memory();
    let used_mem = app.system.used_memory();

    let subtitle = Line::from(vec![
        Span::styled(
            format!(" [{} {}] ", arrow, sort_label),
            Style::default().fg(ORANGE_DARK).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} procs  |  ", total_procs),
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled(
            format!("MEM: {}/{}  |  ", format_bytes(used_mem), format_bytes(total_mem)),
            Style::default().fg(CYAN),
        ),
        Span::styled(
            format!("UP: {}", format_uptime(app.uptime_secs)),
            Style::default().fg(TEXT_DIM),
        ),
    ]);

    let header = Paragraph::new(vec![title, subtitle])
        .style(Style::default().fg(TEXT))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ORANGE_DARK))
                .style(Style::default().bg(BG_DARK)),
        );

    frame.render_widget(header, area);
}

fn render_proc_table(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    let header_cells = [
        "  PID",
        "Name",
        "CPU%",
        "MEM",
        "Status",
    ]
    .iter()
    .map(|h| {
        ratatui::widgets::Cell::from(*h).style(
            Style::default()
                .fg(ORANGE_LIGHT)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells)
        .style(
            Style::default()
                .fg(ORANGE)
                .bg(BG_CARD)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    let rows: Vec<Row> = app
        .processes
        .iter()
        .map(|proc| {
            let name = if proc.cmd.is_empty() {
                proc.name.clone()
            } else {
                proc.cmd.clone()
            };
            let name = if name.len() > 60 {
                format!("{}...", &name[..57])
            } else {
                name
            };

            Row::new(vec![
                format!("{}", proc.pid),
                name,
                format!("{:.1}", proc.cpu_usage),
                format_bytes(proc.memory),
                proc.status.clone(),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(7),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ORANGE_DARK))
                .title(Span::styled(
                    " Processes ",
                    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(BG_DARK)),
        )
        .row_highlight_style(
            Style::default()
                .bg(ORANGE_DARK)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ")
        .column_spacing(2);

    frame.render_stateful_widget(table, area, &mut app.table_state);

    if let Some(info) = app.selected_proc_info() {
        let info_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area)[1];

        let info_para = Paragraph::new(info)
            .style(Style::default().fg(TEXT_DIM))
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(ORANGE_DARK))
                    .title(Span::styled(
                        " Details ",
                        Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
                    )),
            );

        frame.render_widget(info_para, info_area);
    }
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let refresh_indicator = if app.auto_refresh {
        Span::styled(
            "[auto] ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "[manual] ",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )
    };

    let hints = match app.input_mode {
        InputMode::Normal => vec![
            refresh_indicator,
            Span::styled("↑/↓", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" nav  ", Style::default().fg(TEXT_DIM)),
            Span::styled("s", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" search  ", Style::default().fg(TEXT_DIM)),
            Span::styled("k", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" kill  ", Style::default().fg(TEXT_DIM)),
            Span::styled("t", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" sort  ", Style::default().fg(TEXT_DIM)),
            Span::styled("r", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" refresh  ", Style::default().fg(TEXT_DIM)),
            Span::styled("R", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" toggle auto  ", Style::default().fg(TEXT_DIM)),
            Span::styled("q", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" quit", Style::default().fg(TEXT_DIM)),
        ],
        InputMode::Searching => vec![
            Span::styled("Esc", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel  ", Style::default().fg(TEXT_DIM)),
            Span::styled("Enter", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" search", Style::default().fg(TEXT_DIM)),
        ],
        InputMode::ConfirmKill => vec![
            Span::styled("y", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(" kill  ", Style::default().fg(TEXT_DIM)),
            Span::styled("Esc", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(TEXT_DIM)),
        ],
    };

    let footer = Paragraph::new(Line::from(hints))
        .style(Style::default().fg(TEXT))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ORANGE_DARK))
                .style(Style::default().bg(BG_DARK)),
        );

    frame.render_widget(footer, area);
}

fn render_search_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = ratatui::layout::Rect {
        x: area.width / 4,
        y: area.height / 3,
        width: area.width / 2,
        height: 4,
    };

    frame.render_widget(ratatui::widgets::Clear, popup);

    let cursor_char = "▌";
    let before = &app.search_query[..app.search_cursor.min(app.search_query.len())];
    let after = &app.search_query[app.search_cursor.min(app.search_query.len())..];

    let input_line = Line::from(vec![
        Span::raw(before),
        Span::styled(cursor_char, Style::default().fg(ORANGE)),
        Span::raw(after),
    ]);

    let popup_block = Paragraph::new(vec![
        Line::from(Span::styled(
            " Search ",
            Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD),
        )),
        input_line,
    ])
    .style(Style::default().fg(TEXT))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE))
            .style(Style::default().bg(BG_CARD)),
    );

    frame.render_widget(popup_block, popup);
}

fn render_kill_confirm(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = ratatui::layout::Rect {
        x: area.width / 4,
        y: area.height / 3,
        width: area.width / 2,
        height: 5,
    };

    frame.render_widget(ratatui::widgets::Clear, popup);

    let proc_name = if let Some(i) = app.table_state.selected() {
        app.processes.get(i).map(|p| p.name.clone()).unwrap_or_default()
    } else {
        String::new()
    };

    let popup_block = Paragraph::new(vec![
        Line::from(Span::styled(
            " Kill Process ",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("PID: ", Style::default().fg(TEXT_DIM)),
            Span::styled(
                app.selected_pid.to_string(),
                Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::raw(&proc_name),
        ]),
        Line::from(vec![
            Span::styled("y", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm kill", Style::default().fg(TEXT_DIM)),
            Span::styled("  |  ", Style::default().fg(TEXT_DIM)),
            Span::styled("Esc", Style::default().fg(ORANGE_LIGHT).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(TEXT_DIM)),
        ]),
    ])
    .style(Style::default().fg(TEXT))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(RED))
            .style(Style::default().bg(BG_CARD)),
    );

    frame.render_widget(popup_block, popup);
}

fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next();
                        }
                        KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Char('r') => {
                            app.refresh();
                        }
                        KeyCode::Char('s') => {
                            app.start_search();
                        }
                        KeyCode::Char('t') => {
                            app.cycle_sort();
                        }
                        KeyCode::Char('k') => {
                            app.confirm_kill();
                        }
                        KeyCode::Char('R') => {
                            app.toggle_refresh();
                        }
                        _ => {}
                    },
                    InputMode::Searching => match key.code {
                        KeyCode::Esc => {
                            app.cancel_search();
                        }
                        KeyCode::Enter => {
                            app.submit_search();
                        }
                        KeyCode::Char(c) => {
                            app.search_char(c);
                        }
                        KeyCode::Backspace => {
                            app.search_backspace();
                        }
                        KeyCode::Left => {
                            app.search_left();
                        }
                        KeyCode::Right => {
                            app.search_right();
                        }
                        _ => {}
                    },
                    InputMode::ConfirmKill => match key.code {
                        KeyCode::Char('y') => {
                            app.execute_kill();
                        }
                        KeyCode::Esc => {
                            app.cancel_kill();
                        }
                        _ => {}
                    },
                }

                if app.should_quit {
                    break;
                }
            }
        } else if app.auto_refresh {
            app.refresh();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> io::Result<()> {
    run()
}
