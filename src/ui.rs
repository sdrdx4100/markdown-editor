use crate::app::App;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_area);

    draw_editor(f, app, panes[0]);
    draw_preview(f, app, panes[1]);
    draw_status(f, app, status_area);

    if app.show_help {
        draw_help(f, f.area());
    }
}

fn draw_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    app.editor.adjust_scroll(inner_height);

    let scroll_offset = app.editor.scroll_offset;
    let lines: Vec<Line> = app
        .editor
        .lines()
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(inner_height)
        .map(|(i, line)| {
            let line_num = format!("{:4} ", i + 1);
            Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::raw(line.clone()),
            ])
        })
        .collect();

    let editor_block = Block::default()
        .borders(Borders::ALL)
        .title(" Editor ")
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(lines).block(editor_block);
    f.render_widget(paragraph, area);

    let cursor_row = app.editor.cursor_row();
    let cursor_col = app.editor.cursor_col();
    let visible_row = cursor_row.saturating_sub(scroll_offset);
    let line_num_width = 5;
    let x = area.x + 1 + line_num_width + cursor_col as u16;
    let y = area.y + 1 + visible_row as u16;
    if x < area.x + area.width - 1 && y < area.y + area.height - 1 {
        f.set_cursor_position((x, y));
    }
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let content = app.editor.content();
    let lines = render_markdown(&content);

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .border_style(Style::default().fg(Color::Green));

    let paragraph = Paragraph::new(lines)
        .block(preview_block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_markdown(input: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut code_block = false;
    let mut in_list = false;
    let mut list_depth: usize = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
                let (prefix, color) = match level {
                    HeadingLevel::H1 => ("# ", Color::Yellow),
                    HeadingLevel::H2 => ("## ", Color::Cyan),
                    HeadingLevel::H3 => ("### ", Color::Green),
                    _ => ("#### ", Color::White),
                };
                current_spans.push(Span::styled(
                    prefix,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                bold = true;
                let _ = color;
            }
            Event::End(TagEnd::Heading(_)) => {
                bold = false;
                lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                lines.push(Line::raw(""));
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
                lines.push(Line::raw(""));
            }
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::List(_)) => {
                in_list = true;
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                if list_depth == 0 {
                    in_list = false;
                    lines.push(Line::raw(""));
                }
            }
            Event::Start(Tag::Item) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                current_spans.push(Span::styled(
                    format!("{}• ", indent),
                    Style::default().fg(Color::Yellow),
                ));
                let _ = in_list;
            }
            Event::End(TagEnd::Item) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                code_block = true;
                lines.push(Line::styled(
                    "─────────────────────",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::End(TagEnd::CodeBlock) => {
                code_block = false;
                lines.push(Line::styled(
                    "─────────────────────",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::raw(""));
            }
            Event::Code(text) => {
                current_spans.push(Span::styled(
                    text.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Event::Text(text) => {
                let style = if code_block {
                    Style::default().fg(Color::Green)
                } else {
                    let mut s = Style::default();
                    if bold {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    if italic {
                        s = s.add_modifier(Modifier::ITALIC);
                    }
                    s
                };

                if code_block {
                    for line in text.lines() {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            style,
                        ));
                    }
                } else {
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
            }
            Event::Rule => {
                lines.push(Line::styled(
                    "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let file_name = app
        .file_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "[No File]".to_string());

    let status = format!(" {} | {}", file_name, app.status_message);
    let paragraph = Paragraph::new(status).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::raw(""),
        Line::styled(
            "  Markdown Editor - Keyboard Shortcuts  ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::raw("  Ctrl+Q    Quit"),
        Line::raw("  Ctrl+S    Save file"),
        Line::raw("  Ctrl+O    Open output.md"),
        Line::raw("  Ctrl+N    New file"),
        Line::raw("  Ctrl+H    Show this help"),
        Line::raw(""),
        Line::raw("  Arrow keys  Move cursor"),
        Line::raw("  Home / End  Start / End of line"),
        Line::raw("  Ctrl+Home   Start of file"),
        Line::raw("  Ctrl+End    End of file"),
        Line::raw("  Tab         Insert 4 spaces"),
        Line::raw("  Enter       New line"),
        Line::raw("  Backspace   Delete backward"),
        Line::raw("  Delete      Delete forward"),
        Line::raw(""),
        Line::styled(
            "  Press any key to close  ",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let width = 45u16;
    let height = help_text.len() as u16 + 2;
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let popup_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Help ");

    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(help_text).block(block),
        popup_area,
    );
}
