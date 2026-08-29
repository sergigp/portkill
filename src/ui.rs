use crossterm::event::{self, KeyCode, KeyModifiers};
use humansize::{DECIMAL, format_size};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Row, Table, TableState},
};
use sysinfo::System;

use std::{
    io,
    time::{Duration, Instant},
};

use crate::port_process::{self, PortProcess};

pub struct UI {
    table: TableState,
}

impl UI {
    pub fn init() -> UI {
        let mut table_state = TableState::default();

        table_state.select_first();

        Self { table: table_state }
    }

    pub fn draw<F>(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        mut sys: System,
        refetch: F,
    ) -> io::Result<()>
    where
        F: Fn(&mut System) -> Vec<PortProcess>,
    {
        let tick_rate = Duration::from_secs(1);
        let mut last_tick = Instant::now();
        let mut killable_processes = refetch(&mut sys);

        loop {
            terminal.draw(|frame| self.render_table(frame, &killable_processes))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)?
                && let Some(key) = event::read()?.as_key_press_event()
            {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => self.table.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.table.select_previous(),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break Ok(());
                    }
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    KeyCode::Enter => {
                        if let Some(selected_index) = self.table.selected()
                            && let Some(process_to_kill) = killable_processes.get(selected_index)
                        {
                            port_process::kill_port_process(process_to_kill);
                        }

                        break Ok(());
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                killable_processes = refetch(&mut sys);
                last_tick = Instant::now();
            }
        }
    }

    pub fn render_table(&mut self, frame: &mut Frame, killable_processes: &[PortProcess]) {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1);
        let [top, main] = frame.area().layout(&layout);

        let title = Line::from_iter([
            Span::from("Port Kill").style(Style::new().bold()),
            Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
        ]);
        frame.render_widget(title.centered(), top);

        let header = Row::new([
            "Command",
            "User",
            "CPU %",
            "Memory (MB)",
            "Running for",
            "Ports",
            "Path",
        ])
        .style(Style::new().bold())
        .bottom_margin(1);

        let rows = killable_processes.iter().map(|p| {
            let cpu = p.cpu;
            let mut ports = p
                .ports
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            ports.sort();

            Row::new([
                p.command.clone(),
                p.user.clone(),
                format!("{cpu:.0}"),
                format_size(p.memory, DECIMAL),
                p.running_since.clone(),
                ports.join(" "),
                p.path.clone(),
            ])
        });

        let widths = [
            Constraint::Percentage(15),
            Constraint::Percentage(8),
            Constraint::Percentage(4),
            Constraint::Percentage(4),
            Constraint::Percentage(7),
            Constraint::Percentage(14),
            Constraint::Percentage(48),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .style(Color::White)
            .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        frame.render_stateful_widget(table, main, &mut self.table);
    }
}
