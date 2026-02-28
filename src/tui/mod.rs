mod popup_widget;

use anyhow::Result;

use crossterm::{
    cursor::Show,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    text::Text,
    widgets::{Clear, Paragraph, Widget},
    Terminal,
};
use reqwest::get;

use std::{future::Future, io, pin::Pin, time::Duration};

use crate::Parser;

#[derive(Debug, Copy, Clone, Default)]
enum Mode {
    #[default]
    Normal,
    Insert,
}

enum Command {
    Goto,
    None,
    Quit,
}

#[derive(Debug)]
struct State {
    is_running: bool,
    mode: Mode,
    url: String,
    elements: Option<crate::parser::Node>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_running: true,
            mode: Mode::default(),
            url: String::default(),
            elements: None,
        }
    }
}

#[inline(always)]
fn restore_terminal_state() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    Ok(())
}

pub struct App {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl App {
    pub fn new() -> Result<App> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub async fn run(&mut self) -> Result<()> {
        let state = State::default();
        run_app(&mut self.terminal, state).await?;
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        restore_terminal_state().expect("Failed to restore terminal state");
    }
}

fn handle_input(state: &mut State, event: CEvent) -> Command {
    let CEvent::Key(key) = event else {
        return Command::None;
    };

    let mode = state.mode;
    match mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => Command::Quit,
            KeyCode::Char('i') => {
                state.mode = Mode::Insert;
                Command::None
            }
            KeyCode::Char(':') => {
                state.mode = Mode::Insert;
                Command::None
            }
            _ => Command::None,
        },
        Mode::Insert => match key.code {
            KeyCode::Esc => {
                state.mode = Mode::Normal;
                Command::None
            }
            KeyCode::Char(c) => {
                state.url.push(c);
                Command::None
            }
            KeyCode::Enter => {
                state.mode = Mode::Normal;
                Command::Goto
            }
            KeyCode::Backspace => {
                // TODO: once we make the cursor moveable we will need to account for that here.
                // So pressing i put you in Insert mode but really that is insert for the
                // query mode and then if we want app commands :
                // Probably obviouse.
                state.url.pop();
                Command::None
            }
            _ => Command::None,
        },
    }
}

fn draw_ui(f: &mut ratatui::Frame, state: &State) {
    match state.mode {
        Mode::Normal => {
            let Some(elements) = &state.elements else {
                f.render_widget(
                    Text::from(format!("{:?}", state.elements)),
                    ratatui::layout::Rect::new(0, 0, f.area().width, f.area().height),
                );
                return;
            };

            let mut lines = Vec::new();
            render_node(elements, &mut lines);

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, f.area());
        }
        Mode::Insert => {
            use crate::tui::popup_widget::PopupWidget;
            let size = f.area();

            let popup = PopupWidget::new(&state.url)
                .with_closing_message(false)
                .with_percent_width(50)
                .with_height(4);
            let area = popup.get_area(size);
            f.render_widget(Clear, area);
            popup.render(size, f.buffer_mut());
        }
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: State,
) -> Result<()> {
    while state.is_running {
        terminal.draw(|f| draw_ui(f, &state))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let ev = event::read()?;
        let cmd = handle_input(&mut state, ev);
        handle_command(cmd, &mut state).await?;
    }
    Ok(())
}

fn handle_command<'a>(
    cmd: Command,
    state: &'a mut State,
) -> Pin<Box<dyn Future<Output = io::Result<()>> + 'a>> {
    Box::pin(async move {
        match cmd {
            Command::Goto => {
                let url = &state.url;
                match get(url).await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            eprintln!("Failed to fetch url: {}", response.status());
                            state.is_running = false;
                            return Ok(());
                        }
                        match response.text().await {
                            Ok(text) => {
                                let node = Parser::parse(text);
                                state.elements = Some(node);
                            }
                            Err(e) => {
                                eprintln!("Failed to parse url: {}", e);
                                state.is_running = false;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch url: {}", e);
                        state.is_running = false;
                    }
                }
            }
            Command::Quit => state.is_running = false,
            Command::None => {}
        }
        Ok(())
    })
}

use crate::parser::{Node, NodeType};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

fn render_node(node: &Node, lines: &mut Vec<Line>) {
    match &node.node_type {
        NodeType::Text(text) => {
            lines.push(Line::from(text.clone()));
        }

        NodeType::Element(element) => {
            match element.tag_name.as_str() {
                "h1" => {
                    for child in &node.children {
                        if let NodeType::Text(text) = &child.node_type {
                            lines.push(Line::from(Span::styled(
                                text.clone(),
                                Style::default().add_modifier(Modifier::BOLD),
                            )));
                        }
                    }
                    lines.push(Line::from("")); // spacing
                }

                "p" | "div" | "body" | "html" => {
                    for child in &node.children {
                        render_node(child, lines);
                    }
                    lines.push(Line::from(""));
                }

                "em" => {
                    for child in &node.children {
                        if let NodeType::Text(text) = &child.node_type {
                            lines.push(Line::from(Span::styled(
                                text.clone(),
                                Style::default().add_modifier(Modifier::ITALIC),
                            )));
                        }
                    }
                }

                _ => {
                    for child in &node.children {
                        render_node(child, lines);
                    }
                }
            }
        }
    }
}
