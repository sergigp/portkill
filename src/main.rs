use std::io;

mod port_process;
mod ui;

fn main() -> io::Result<()> {
    let killable_processes = port_process::get_port_processes();

    let mut ui = ui::UI::init();

    ratatui::run(|terminal| ui.draw(terminal, &killable_processes))
}
