use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use super::escape;

pub fn to_rml(md: &str) -> String {
    let mut out = String::new();
    let mut lists: Vec<Option<u64>> = Vec::new();
    for event in Parser::new(md) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => out.push_str("<p>"),
                Tag::Heading { level, .. } => out.push_str(heading(level, false)),
                Tag::List(start) => {
                    out.push_str(if start.is_some() { "<ol>" } else { "<ul>" });
                    lists.push(start);
                }
                Tag::Item => {
                    let marker = match lists.last_mut() {
                        Some(Some(n)) => {
                            let m = format!("{n}.");
                            *n += 1;
                            m
                        }
                        _ => "\u{2022}".to_string(),
                    };
                    out.push_str(&format!("<li><span class=\"marker\">{marker}</span>"));
                }
                Tag::Emphasis => out.push_str("<em>"),
                Tag::Strong => out.push_str("<strong>"),
                Tag::BlockQuote(_) => out.push_str("<blockquote>"),
                Tag::CodeBlock(_) => out.push_str("<pre><code>"),
                Tag::Link { .. } => out.push_str("<a>"),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => out.push_str("</p>"),
                TagEnd::Heading(level) => out.push_str(heading(level, true)),
                TagEnd::List(ordered) => {
                    lists.pop();
                    out.push_str(if ordered { "</ol>" } else { "</ul>" });
                }
                TagEnd::Item => out.push_str("</li>"),
                TagEnd::Emphasis => out.push_str("</em>"),
                TagEnd::Strong => out.push_str("</strong>"),
                TagEnd::BlockQuote(_) => out.push_str("</blockquote>"),
                TagEnd::CodeBlock => out.push_str("</code></pre>"),
                TagEnd::Link => out.push_str("</a>"),
                _ => {}
            },
            Event::Text(t) => out.push_str(&escape(&t)),
            Event::Code(t) => out.push_str(&format!("<code>{}</code>", escape(&t))),
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push_str("<br/>"),
            Event::Rule => out.push_str("<hr/>"),
            _ => {}
        }
    }
    out
}

fn heading(level: HeadingLevel, close: bool) -> &'static str {
    match (level, close) {
        (HeadingLevel::H1, false) => "<h1>",
        (HeadingLevel::H1, true) => "</h1>",
        (HeadingLevel::H2, false) => "<h2>",
        (HeadingLevel::H2, true) => "</h2>",
        (_, false) => "<h3>",
        (_, true) => "</h3>",
    }
}
