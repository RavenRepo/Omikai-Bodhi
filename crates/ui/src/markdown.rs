use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownSegment {
    Text(String),
    Bold(String),
    Code(String),
    CodeBlock { lang: Option<String>, content: String },
}

pub fn parse_markdown(text: &str) -> Vec<MarkdownSegment> {
    let mut segments = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current_text = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang: Option<String> = None;

    while let Some(c) = chars.next() {
        if in_code_block {
            if c == '`' && chars.peek() == Some(&'`') {
                chars.next();
                if chars.peek() == Some(&'`') {
                    chars.next();
                    segments.push(MarkdownSegment::CodeBlock {
                        lang: code_block_lang.take(),
                        content: code_block_content.trim().to_string(),
                    });
                    code_block_content.clear();
                    in_code_block = false;
                    continue;
                }
                code_block_content.push_str("``");
            } else {
                code_block_content.push(c);
            }
            continue;
        }

        match c {
            '`' => {
                if chars.peek() == Some(&'`') {
                    chars.next();
                    if chars.peek() == Some(&'`') {
                        chars.next();
                        if !current_text.is_empty() {
                            segments.push(MarkdownSegment::Text(current_text.clone()));
                            current_text.clear();
                        }
                        in_code_block = true;
                        let mut lang = String::new();
                        while let Some(&ch) = chars.peek() {
                            if ch == '\n' || ch == ' ' {
                                chars.next();
                                break;
                            }
                            lang.push(chars.next().unwrap());
                        }
                        code_block_lang = if lang.is_empty() { None } else { Some(lang) };
                    } else {
                        current_text.push_str("``");
                    }
                } else {
                    if !current_text.is_empty() {
                        segments.push(MarkdownSegment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut code = String::new();
                    for ch in chars.by_ref() {
                        if ch == '`' {
                            break;
                        }
                        code.push(ch);
                    }
                    if !code.is_empty() {
                        segments.push(MarkdownSegment::Code(code));
                    }
                }
            }
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if !current_text.is_empty() {
                        segments.push(MarkdownSegment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut bold = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold.push(ch);
                    }
                    if !bold.is_empty() {
                        segments.push(MarkdownSegment::Bold(bold));
                    }
                } else {
                    current_text.push(c);
                }
            }
            _ => {
                current_text.push(c);
            }
        }
    }

    if in_code_block && !code_block_content.is_empty() {
        segments.push(MarkdownSegment::CodeBlock {
            lang: code_block_lang,
            content: code_block_content,
        });
    } else if !current_text.is_empty() {
        segments.push(MarkdownSegment::Text(current_text));
    }

    segments
}

pub fn render_markdown_line<'a>(text: &'a str, base_color: Color, code_color: Color) -> Line<'a> {
    let segments = parse_markdown(text);
    let spans: Vec<Span<'a>> = segments
        .into_iter()
        .map(|seg| match seg {
            MarkdownSegment::Text(s) => Span::styled(s, Style::default().fg(base_color)),
            MarkdownSegment::Bold(s) => {
                Span::styled(s, Style::default().fg(base_color).add_modifier(Modifier::BOLD))
            }
            MarkdownSegment::Code(s) => {
                Span::styled(format!("`{}`", s), Style::default().fg(code_color))
            }
            MarkdownSegment::CodeBlock { content, .. } => {
                Span::styled(content, Style::default().fg(code_color))
            }
        })
        .collect();
    Line::from(spans)
}

#[allow(dead_code)]
pub fn is_code_block_start(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

#[allow(dead_code)]
pub fn is_code_block_end(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "```"
        || (trimmed.starts_with("```") && trimmed.len() > 3 && !trimmed[3..].contains('`'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let segments = parse_markdown("Hello world");
        assert_eq!(segments, vec![MarkdownSegment::Text("Hello world".to_string())]);
    }

    #[test]
    fn test_parse_bold() {
        let segments = parse_markdown("Hello **bold** world");
        assert_eq!(
            segments,
            vec![
                MarkdownSegment::Text("Hello ".to_string()),
                MarkdownSegment::Bold("bold".to_string()),
                MarkdownSegment::Text(" world".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_inline_code() {
        let segments = parse_markdown("Use `code` here");
        assert_eq!(
            segments,
            vec![
                MarkdownSegment::Text("Use ".to_string()),
                MarkdownSegment::Code("code".to_string()),
                MarkdownSegment::Text(" here".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_code_block() {
        let segments = parse_markdown("```rust\nfn main() {}\n```");
        assert_eq!(
            segments,
            vec![MarkdownSegment::CodeBlock {
                lang: Some("rust".to_string()),
                content: "fn main() {}".to_string(),
            }]
        );
    }

    #[test]
    fn test_is_code_block_start() {
        assert!(is_code_block_start("```rust"));
        assert!(is_code_block_start("   ```"));
        assert!(!is_code_block_start("hello ```"));
    }
}
