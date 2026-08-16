use std::{collections::HashMap, num::ParseIntError, os::unix::process, process::Command};

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

#[derive(Debug)]
struct NetworkProcess {
    command: String,
    pid: u32,
    user: String,
    protocol: String, // tcp/udp
    port: u16,
}

#[derive(Debug)]
struct KillableProcess {
    pid: u32,
    command: String,
    ports: Vec<u16>,
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))

    // let output = Command::new("lsof")
    //     .args(["-i", "-P", "-n"])
    //     .output()
    //     .expect("Error ocurred running lsof");

    // if !output.status.success() {
    //     let error = String::from_utf8_lossy(&output.stderr);
    //     panic!("Error: {error}")
    // }

    // let content = String::from_utf8(output.stdout).expect("Unable to parse lsof response");

    // let network_processes = content
    //     .lines()
    //     .skip(1)
    //     .filter_map(|line| {
    //         let fields: Vec<&str> = line.split_whitespace().collect();
    //         let name = *fields.get(8)?;

    //         if !is_port_holder(name) {
    //             return None;
    //         }
    //         Some(NetworkProcess {
    //             command: fields.first()?.to_string(),
    //             pid: fields
    //                 .get(1)?
    //                 .parse()
    //                 .inspect_err(|e| eprintln!("bad pid {:?}: {e}", fields.get(1)))
    //                 .ok()?,
    //             user: fields.get(2)?.to_string(),
    //             protocol: fields.get(7)?.to_string(),
    //             port: parse_local_port(name)
    //                 .inspect_err(|e| eprintln!("bad port {:?}: {e}", name))
    //                 .ok()?,
    //         })
    //     })
    //     .collect::<Vec<NetworkProcess>>();

    // let killable_processes = network_processes
    //     .into_iter()
    //     .fold(
    //         HashMap::<u32, Vec<NetworkProcess>>::new(),
    //         |mut grouped, p| {
    //             grouped.entry(p.pid).or_default().push(p);
    //             grouped
    //         },
    //     )
    //     .into_values()
    //     .map(|procs| {
    //         let first = procs.first().expect("empty group");
    //         KillableProcess {
    //             pid: first.pid,
    //             command: first.command.clone(),
    //             ports: procs.iter().map(|p| p.port).collect(),
    //         }
    //     })
    //     .collect::<Vec<_>>();

    // dbg!(killable_processes);
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec!["Value: 0".into()])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

fn is_port_holder(name: &str) -> bool {
    if name.contains("->") {
        // Outbound connection
        return false;
    }
    let Some(addr) = name.split_whitespace().next() else {
        return false;
    };
    let Some(port) = addr.rsplit(':').next() else {
        return false;
    };

    port != "*" && !port.is_empty()
}

fn parse_local_port(name: &str) -> Result<u16, ParseIntError> {
    let addr = name.split_whitespace().next().unwrap();
    addr.rsplit(':').next().unwrap().parse()
}
