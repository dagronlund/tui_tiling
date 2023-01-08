use core::panic;
use std::io::{stdout, Stdout, Write};
use std::thread;
use std::time::{self, Duration, Instant};

use crossbeam::channel::{unbounded, Sender};

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    QueueableCommand, Result as CrosstermResult,
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Direction, Rect},
    Terminal,
};

use tui_tiling::{
    component::{Component, ComponentBase, ComponentBaseWidget, ComponentWidget},
    container::{list::ContainerList, Container, ContainerChild},
    ResizeError,
};

pub struct TestComponentWidget {
    print_last: bool,
    mouse_last: Option<(u16, u16, MouseEventKind)>,
    key_last: Option<KeyEvent>,
}

impl TestComponentWidget {
    pub fn new(print_last: bool) -> Self {
        Self {
            print_last,
            mouse_last: None,
            key_last: None,
        }
    }
}

impl ComponentWidget for TestComponentWidget {
    fn handle_mouse(&mut self, x: u16, y: u16, e: MouseEventKind) -> bool {
        self.mouse_last = Some((x, y, e));
        true
    }

    fn handle_key(&mut self, e: KeyEvent) -> bool {
        self.key_last = Some(e);
        true
    }

    fn handle_update(&mut self) -> bool {
        false
    }

    fn resize(&mut self, _: u16, _: u16) {}

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        for x in 0..area.width {
            buf.get_mut(area.x + x, area.y).symbol = format!("#");
            buf.get_mut(area.x + x, area.y + area.height - 1).symbol = format!("#");
        }
        for y in 0..area.height {
            buf.get_mut(area.x, area.y + y).symbol = format!("#");
            buf.get_mut(area.x + area.width - 1, area.y + y).symbol = format!("#");
        }

        if !self.print_last {
            return;
        }
        let mouse_msg = format!("{:?}", self.mouse_last);
        let key_msg = format!("{:?}", self.key_last);
        for (i, c) in mouse_msg.chars().into_iter().enumerate() {
            if area.height > 2 && area.width > i as u16 + 2 {
                buf.get_mut(area.x + i as u16 + 1, area.y + 1).symbol = format!("{}", c);
            }
        }
        for (i, c) in key_msg.chars().into_iter().enumerate() {
            if area.height > 3 && area.width > i as u16 + 2 {
                buf.get_mut(area.x + i as u16 + 1, area.y + 2).symbol = format!("{}", c);
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn render_helper(component_base: &mut dyn ComponentBase) -> Buffer {
    let rect = Rect::new(
        0,
        0,
        component_base.get_width(),
        component_base.get_height(),
    );
    let mut buffer = Buffer::empty(rect.clone());
    component_base.render(rect, &mut buffer);
    buffer
}

fn spawn_input_listener(tx: Sender<CrosstermEvent>) {
    thread::spawn(move || loop {
        if event::poll(time::Duration::from_millis(100)).unwrap() {
            tx.send(event::read().unwrap()).unwrap();
        }
    });
}

pub fn get_tui(print_last: bool) -> Result<Box<dyn Container>, ResizeError> {
    let tui = ContainerList::new(
        String::from("horizontal"),
        Direction::Horizontal,
        true,
        0,
        0,
    )
    .from_children(vec![
        ContainerChild::from(
            ContainerList::new(String::from("vertical"), Direction::Vertical, true, 0, 0)
                .from_children(vec![
                    ContainerChild::from(
                        Component::new(
                            String::from("fixed"),
                            1,
                            Box::new(TestComponentWidget::new(print_last)),
                        )
                        .fixed_height(Some(6)),
                    ),
                    ContainerChild::from(Component::new(
                        String::from("a"),
                        1,
                        Box::new(TestComponentWidget::new(print_last)),
                    )),
                    ContainerChild::from(Component::new(
                        String::from("b"),
                        1,
                        Box::new(TestComponentWidget::new(print_last)),
                    )),
                ])?,
        ),
        ContainerChild::from(Component::new(
            String::from("c"),
            1,
            Box::new(TestComponentWidget::new(print_last)),
        )),
    ])?;
    Ok(Box::new(tui))
}

fn setup_terminal() -> CrosstermResult<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.backend_mut().queue(EnableMouseCapture)?;
    terminal.backend_mut().queue(EnterAlternateScreen)?;
    terminal.backend_mut().flush()?;
    terminal.clear()?;
    Ok(terminal)
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> CrosstermResult<()> {
    terminal.backend_mut().queue(DisableMouseCapture)?;
    terminal.backend_mut().queue(LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}

fn cleanup_terminal_force() -> CrosstermResult<()> {
    cleanup_terminal(&mut Terminal::new(CrosstermBackend::new(stdout()))?)
}

fn tui_main_unmanaged(
    mut tui: Box<dyn Container>,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> CrosstermResult<String> {
    let (tx_input, rx_input) = unbounded();
    let mut last_buffer: Option<Buffer> = None;
    spawn_input_listener(tx_input);

    loop {
        let frame_start = Instant::now();

        // Check if the last buffer can be reused
        if let Some(mut last_buffer) = last_buffer {
            if last_buffer.area == terminal.current_buffer_mut().area {
                std::mem::swap(
                    &mut terminal.current_buffer_mut().content,
                    &mut last_buffer.content,
                );
            }
        }

        // Render the next frame
        let next_frame = terminal.draw(|frame| {
            if let Err(err) = tui
                .as_base_mut()
                .resize(frame.size().width, frame.size().height)
            {
                panic!("Resizing Error! ({err:?})");
            }
            frame.render_stateful_widget(
                ComponentBaseWidget::from(tui.as_base_mut()),
                frame.size(),
                &mut (),
            );
        })?;
        last_buffer = Some(next_frame.buffer.clone());

        let mut done_msg = None;
        while !rx_input.is_empty() {
            match rx_input.recv().unwrap() {
                CrosstermEvent::Key(key) => {
                    if key.clone().code == KeyCode::Char('q') {
                        done_msg = Some(String::from("User quit!"));
                    } else {
                        tui.as_base_mut().handle_key(key);
                    }
                }
                CrosstermEvent::Mouse(event) => {
                    tui.as_base_mut()
                        .handle_mouse(event.column, event.row, Some(event.kind));
                }
                CrosstermEvent::Resize(columns, rows) => {
                    if let Err(err) = tui.as_base_mut().resize(rows, columns) {
                        panic!("Resizing Error! ({err:?})");
                    }
                }
                CrosstermEvent::FocusGained
                | CrosstermEvent::FocusLost
                | CrosstermEvent::Paste(_) => {}
            }
        }

        // Check if done
        if let Some(msg) = done_msg {
            cleanup_terminal(terminal)?;
            return Ok(msg);
        }

        // Sleep for unused frame time
        let frame_target = Duration::from_millis(20);
        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < frame_target {
            thread::sleep(frame_target - frame_start.elapsed());
        }
    }
}

// Dark magic to capture backtraces from nalu_main, cleanup the terminal state,
// and then print the backtrace on the normal terminal
use backtrace::Backtrace;
use std::cell::RefCell;

thread_local! {
    static BACKTRACE: RefCell<Option<Backtrace>> = RefCell::new(None);
}

pub fn tui_main() -> CrosstermResult<()> {
    if !stdout().is_tty() {
        panic!("Error: Cannot open viewer when not TTY!");
    }

    std::panic::set_hook(Box::new(|_| {
        let trace = Backtrace::new();
        BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
    }));

    // Catch any panics and try to cleanup the terminal first
    match std::panic::catch_unwind(|| {
        let tui = get_tui(true).unwrap();
        let mut terminal = setup_terminal().unwrap();
        tui_main_unmanaged(tui, &mut terminal).unwrap()
    }) {
        Ok(msg) => println!("{}", msg),
        Err(e) => {
            cleanup_terminal_force()?;
            let backtrace = BACKTRACE.with(|b| b.borrow_mut().take()).unwrap();
            panic!("Error:\n{:?}\n{:?}", e, backtrace);
        }
    }

    Ok(())
}
