use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub(crate) struct PopupWidget<'a> {
    message: &'a str,
    width_percent: Option<u16>,
    height_percent: Option<u16>,
    width: Option<u16>,
    height: Option<u16>,
    title: Option<String>,
    dont_show_closing_message: bool,
}

impl<'a> PopupWidget<'a> {
    pub(crate) fn new(message: &'a str) -> Self {
        Self {
            message,
            width_percent: None,
            height_percent: None,
            width: None,
            height: None,
            title: None,
            dont_show_closing_message: true,
        }
    }

    pub fn with_closing_message(mut self, flag: bool) -> Self {
        self.dont_show_closing_message = flag;
        self
    }

    pub fn _with_width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_percent_width(mut self, percent: u16) -> Self {
        self.width_percent = Some(percent);
        self
    }

    pub fn _with_percent_height(mut self, percent: u16) -> Self {
        self.height_percent = Some(percent);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    fn calculate_width(&self, area: Rect) -> u16 {
        self.width_percent.unwrap_or_else(|| {
            let max_line_length = self
                .message
                .lines()
                .map(|line| line.len())
                .max()
                .unwrap_or(1) as u16;
            ((max_line_length + 4) as f64 / area.width as f64 * 100.0).ceil() as u16
        })
    }

    fn calculate_height(&self, area: Rect) -> u16 {
        self.height_percent.unwrap_or_else(|| {
            let lines = self.message.lines().count() as u16;
            ((lines + 2) as f64 / area.height as f64 * 100.0).ceil() as u16
        })
    }

    pub fn get_area(&self, area: Rect) -> Rect {
        let vertical = if let Some(width) = self.width {
            Layout::horizontal([Constraint::Max(width)]).flex(Flex::Center)
        } else {
            let width_percent = self.calculate_width(area);
            Layout::horizontal([Constraint::Percentage(width_percent)]).flex(Flex::Center)
        };
        let horizontal = if let Some(height) = self.height {
            Layout::vertical([Constraint::Max(height)]).flex(Flex::Center)
        } else {
            let height_percent = self.calculate_height(area);
            Layout::vertical([Constraint::Percentage(height_percent)]).flex(Flex::Center)
        };

        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);

        area
    }
}

impl Widget for PopupWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.get_area(area);
        let mut lines = self
            .message
            .lines()
            .map(|line| Line::from(vec![Span::raw(line)]))
            .collect::<Vec<_>>();
        if self.dont_show_closing_message {
            lines.push(Line::from(vec![Span::styled(
                "Press any key to close",
                Style::default().fg(Color::Green),
            )]));
        }
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.title.unwrap_or_default()),
        );
        paragraph.render(area, buf);
    }
}
