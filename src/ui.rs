use crossterm::event::{self, KeyCode, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Row, Table, TableState},
};

use std::io;

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

    pub fn draw(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        killable_processes: &[PortProcess],
    ) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.render_table(frame, killable_processes))?;
            if let Some(key) = event::read()?.as_key_press_event() {
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

        let header = Row::new(["Command", "User", "Ports"])
            .style(Style::new().bold())
            .bottom_margin(1);

        let rows = killable_processes.iter().map(|p| {
            Row::new([
                p.command.clone(),
                p.user.clone(),
                p.ports
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(" "),
            ])
        });

        let widths = [
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(70),
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
