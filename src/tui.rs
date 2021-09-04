use std::{io, sync::mpsc::Receiver, time::Duration};

use anyhow::Result;
use chrono::offset::Local;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Row, Table},
    Terminal,
};

use crate::{
    handlers::{config::CompleteConfig, data::Data},
    utils::{app::App, event},
};

pub fn tui(config: CompleteConfig, mut app: App, rx: Receiver<Data>) -> Result<()> {
    let events = event::Events::with_config(event::Config {
        exit_key: Key::Esc,
        tick_rate: Duration::from_millis(config.terminal.tick_delay),
    });

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let date_format_length = Local::now()
        .format(config.frontend.date_format.as_str())
        .to_string()
        .len() as u16;

    let table_width = &[
        Constraint::Length(date_format_length),
        Constraint::Length(config.frontend.maximum_username_length),
        Constraint::Min(1),
    ];

    loop {
        if let Ok(info) = rx.try_recv() {
            app.messages.push(info);
        }

        terminal.draw(|f| {
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(1)].as_ref())
                .split(f.size());

            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(table_width.as_ref())
                .split(f.size());

            let all_messages = app.messages.clone();

            let chunk_height = vertical_chunks[0].height as usize - 4;
            let chunk_width = horizontal_chunks[2].width as usize - 4;

            let message_amount = all_messages.len();

            let mut rendered_messages = all_messages;

            if rendered_messages.len() >= chunk_height {
                rendered_messages = rendered_messages[message_amount - chunk_height..].to_owned();
            }

            let mut final_rendered_messages: Vec<Data> = Vec::new();

            for msg_data in rendered_messages {
                let new_data = msg_data.wrap_message(chunk_width);
                for some_data in new_data {
                    final_rendered_messages.push(some_data);
                }
            }

            let table = Table::new(
                final_rendered_messages
                    .iter()
                    .map(|m| Row::new(m.to_vec()))
                    .collect::<Vec<Row>>(),
            )
            .style(Style::default().fg(Color::White))
            .header(
                Row::new(vec!["Time", "User", "Message content"])
                    .style(Style::default().fg(Color::Yellow))
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("[ Table of messages ]"),
            )
            .widths(table_width)
            .column_spacing(1);

            f.render_widget(table, vertical_chunks[0]);
        })?;

        if let event::Event::Input(input) = events.next()? {
            if let Key::Esc = input {
                break;
            }
        }
    }

    Ok(())
}
