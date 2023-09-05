use crate::db::dao::History;
use crate::ui::run_ui::UIRunner;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::io;

pub struct HistoryItem {
    pub action_name: String,
    pub url: String,
    pub status_code: String,
    pub duration: String,
    pub timestamp: String,
}

impl HistoryItem {
    pub fn to_cells(&self) -> Vec<String> {
        vec![
            self.timestamp.clone(),
            self.action_name.clone(),
            self.url.clone(),
            self.status_code.clone(),
            self.duration.clone(),
        ]
    }
}

pub(crate) struct HistoryUI {
    histories: Vec<History>,
    state: TableState,
}

impl HistoryUI {
    pub fn new(histories: Vec<History>) -> Self {
        Self {
            histories,
            state: TableState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.histories.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.histories.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn build_ui(&self) -> impl StatefulWidget<State = TableState> {
        //, app: &mut App) {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default().bg(Color::Blue);
        let header_cells = ["timestamp", "action name", "url", "status code", "duration"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = self.histories.iter().map(|item| {
            let item = HistoryItem {
                action_name: item.action_name.clone(),
                url: item.url.clone(),
                status_code: item.status_code.to_string(),
                duration: item.duration.to_string(),
                timestamp: item
                    .timestamp
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or("".to_string()),
            };

            Row::new(item.to_cells()).bottom_margin(1)
        });
        let t = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Table"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]);
        t
    }
}

impl UIRunner for HistoryUI {
    fn handle_event(&mut self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Down => self.next(),
                    KeyCode::Up => self.previous(),
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    fn ui<B: Backend>(&mut self, f: &mut Frame<B>) {
        let t = self.build_ui();
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(f.size());
        f.render_stateful_widget(t, rects[0], &mut self.state);
    }
}
