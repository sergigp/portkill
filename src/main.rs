use std::io;
use sysinfo::System;

use crate::port_process::PortProcess;

mod port_process;
mod ui;

fn main() -> io::Result<()> {
    let f = |sys: &mut System| -> Vec<PortProcess> { port_process::get_port_processes(sys) };

    let system = System::new();

    let mut ui = ui::UI::init();
    ratatui::run(|terminal| ui.draw(terminal, system, f))
}
