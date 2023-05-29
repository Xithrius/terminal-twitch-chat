use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Row, Table},
    Frame,
};

use crate::{
    emotes::Emotes,
    handlers::config::SharedCompleteConfig,
    ui::components::Component,
    utils::text::{title_spans, TitleStyle},
};

#[derive(Debug, Clone)]
pub struct DebugWidget {
    config: SharedCompleteConfig,
    focused: bool,
}

impl DebugWidget {
    pub fn new(config: SharedCompleteConfig) -> Self {
        Self {
            config,
            focused: false,
        }
    }

    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn toggle_focus(&mut self) {
        self.focused = !self.focused;
    }
}

impl Component for DebugWidget {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect, _emotes: Option<Emotes>) {
        // TODO: Add more debug stuff
        let config = self.config.borrow();

        let rows = vec![Row::new(vec!["Current channel", &config.twitch.channel])];

        let title_binding = [TitleStyle::Single("Debug")];

        let table = Table::new(rows)
            .block(
                Block::default()
                    .title(title_spans(
                        &title_binding,
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(self.config.borrow().frontend.border_type.clone().into()),
            )
            .widths(&[Constraint::Length(10), Constraint::Length(10)]);

        f.render_widget(Clear, area);
        f.render_widget(table, area);
    }
}
