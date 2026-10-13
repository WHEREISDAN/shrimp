//! Renders edited text for pasting: plain text always, and HTML when there is
//! structure worth keeping (lists from the LLM, formatting in snippet
//! expansions). Inline markup (`**bold**`, `*italic*`, `[text](url)`) is only
//! read inside snippet expansions, so dictated `snake_case` stays literal.

/// Marks where a snippet expansion starts and ends in the text.
pub const SNIPPET_START: char = '\u{e000}';
pub const SNIPPET_END: char = '\u{e001}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub plain: String,
    pub html: Option<String>,
}

/// Strips the snippet markers and renders inline markup to plain text.
pub fn plain(text: &str) -> String {
    render(text).plain
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Numbered,
}

enum Block {
    Paragraph(Vec<String>),
    List(ListKind, Vec<String>),
}

pub fn render(text: &str) -> Rendered {
    let blocks = blocks(text);
    let mut plain_lines = Vec::new();
    let mut html = String::new();
    let mut structured = false;
    let mut inside = false;
    for (index, block) in blocks.iter().enumerate() {
        match block {
            Block::Paragraph(lines) => {
                if index > 0 {
                    plain_lines.push(String::new());
                    if matches!(blocks[index - 1], Block::Paragraph(_)) {
                        html.push_str("<br><br>");
                    }
                }
                let rendered: Vec<Inline> =
                    lines.iter().map(|line| inline(line, &mut inside)).collect();
                structured |= rendered.iter().any(|line| line.marked);
                plain_lines.extend(rendered.iter().map(|line| line.plain.clone()));
                let body: Vec<&str> = rendered.iter().map(|line| line.html.as_str()).collect();
                // No <p>: apps turn it into a trailing paragraph break, which
                // would split a sentence pasted mid-line.
                html.push_str(&body.join("<br>"));
            }
            Block::List(kind, items) => {
                structured = true;
                if index > 0 && matches!(blocks[index - 1], Block::List(..)) {
                    plain_lines.push(String::new());
                }
                let tag = if *kind == ListKind::Numbered {
                    "ol"
                } else {
                    "ul"
                };
                html.push_str(&format!("<{tag}>"));
                for (number, item) in items.iter().enumerate() {
                    let rendered = inline(item, &mut inside);
                    let marker = match kind {
                        ListKind::Numbered => format!("{}. ", number + 1),
                        ListKind::Bullet => "- ".into(),
                    };
                    plain_lines.push(format!("{marker}{}", rendered.plain));
                    html.push_str(&format!("<li>{}</li>", rendered.html));
                }
                html.push_str(&format!("</{tag}>"));
            }
        }
    }
    let html = structured.then(|| format!("<meta charset=\"utf-8\">{html}"));
    Rendered {
        plain: plain_lines.join("\n"),
        html,
    }
}

/// Groups lines into paragraphs (split by blank lines) and lists.
fn blocks(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let flush = |paragraph: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(std::mem::take(paragraph)));
        }
    };
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if let Some((kind, item)) = list_item(line) {
            flush(&mut paragraph, &mut blocks);
            match blocks.last_mut() {
                Some(Block::List(last, items)) if *last == kind => items.push(item),
                _ => blocks.push(Block::List(kind, vec![item])),
            }
            continue;
        }
        paragraph.push(line.to_owned());
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

/// A list line: its kind and the item text. Snippet markers before the
/// bullet stay with the item, so the expansion keeps its formatting.
fn list_item(line: &str) -> Option<(ListKind, String)> {
    let trimmed = line.trim_start_matches([' ', SNIPPET_START]);
    let markers: String = line[..line.len() - trimmed.len()]
        .chars()
        .filter(|c| *c == SNIPPET_START)
        .collect();
    let item = |text: &str| format!("{markers}{}", text.trim());
    for bullet in ["- ", "* ", "• "] {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return Some((ListKind::Bullet, item(rest)));
        }
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if (1..=3).contains(&digits) {
        let rest = &trimmed[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some((ListKind::Numbered, item(rest)));
        }
    }
    None
}

struct Inline {
    plain: String,
    html: String,
    /// Inline markup was rendered, so HTML is worth pasting.
    marked: bool,
}

/// Renders one line. `inside` tracks whether we are in a snippet expansion,
/// which can span lines.
fn inline(line: &str, inside: &mut bool) -> Inline {
    let chars: Vec<char> = line.chars().collect();
    let mut plain = String::new();
    let mut html = String::new();
    let mut marked = false;
    let mut bold = false;
    let mut italic: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            SNIPPET_START => *inside = true,
            SNIPPET_END => {
                *inside = false;
                if bold {
                    html.push_str("</b>");
                    bold = false;
                }
                if italic.take().is_some() {
                    html.push_str("</i>");
                }
            }
            '\\' if *inside && i + 1 < chars.len() => {
                i += 1;
                plain.push(chars[i]);
                push_escaped(&mut html, chars[i]);
            }
            '*' if *inside && chars.get(i + 1) == Some(&'*') => {
                html.push_str(if bold { "</b>" } else { "<b>" });
                bold = !bold;
                marked = true;
                i += 1;
            }
            '*' | '_' if *inside && (italic == Some(c) || opens_italic(&chars, i)) => {
                if italic == Some(c) {
                    html.push_str("</i>");
                    italic = None;
                } else {
                    html.push_str("<i>");
                    italic = Some(c);
                }
                marked = true;
            }
            '[' if *inside => match link(&chars, i) {
                Some((text, url, end)) => {
                    if text == url {
                        plain.push_str(&url);
                    } else {
                        plain.push_str(&format!("{text} ({url})"));
                    }
                    html.push_str(&format!(
                        "<a href=\"{}\">{}</a>",
                        escape_html(&url),
                        escape_html(&text)
                    ));
                    marked = true;
                    i = end;
                }
                None => {
                    plain.push(c);
                    push_escaped(&mut html, c);
                }
            },
            c => {
                plain.push(c);
                push_escaped(&mut html, c);
            }
        }
        i += 1;
    }
    if bold {
        html.push_str("</b>");
    }
    if italic.is_some() {
        html.push_str("</i>");
    }
    Inline {
        plain,
        html,
        marked,
    }
}

/// `*` or `_` opens emphasis when a closing one follows on the same line and
/// the next character is not a space.
fn opens_italic(chars: &[char], i: usize) -> bool {
    let c = chars[i];
    let next_ok = chars
        .get(i + 1)
        .is_some_and(|n| !n.is_whitespace() && *n != c);
    let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
    next_ok && before_ok && chars[i + 2..].contains(&c)
}

/// `[text](url)` starting at `i`: the text, the url and the index of `)`.
fn link(chars: &[char], i: usize) -> Option<(String, String, usize)> {
    let close = i + chars[i..].iter().position(|c| *c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = close + 1 + chars[close + 1..].iter().position(|c| *c == ')')?;
    let text: String = chars[i + 1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect();
    let safe = ["http://", "https://", "mailto:", "tel:"]
        .iter()
        .any(|scheme| url.starts_with(scheme));
    (safe && !text.is_empty()).then_some((text, url, end))
}

fn push_escaped(html: &mut String, c: char) {
    match c {
        '&' => html.push_str("&amp;"),
        '<' => html.push_str("&lt;"),
        '>' => html.push_str("&gt;"),
        '"' => html.push_str("&quot;"),
        c => html.push(c),
    }
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        push_escaped(&mut out, c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(text: &str) -> String {
        format!("{SNIPPET_START}{text}{SNIPPET_END}")
    }

    #[test]
    fn simple_text_is_plain_only() {
        let rendered = render("Hello there.\nSecond line.\n\nNew paragraph.");
        assert_eq!(
            rendered.plain,
            "Hello there.\nSecond line.\n\nNew paragraph."
        );
        assert_eq!(rendered.html, None);
    }

    #[test]
    fn dictated_markdown_characters_stay_literal() {
        let rendered = render("Rename snake_case_name to *this*");
        assert_eq!(rendered.plain, "Rename snake_case_name to *this*");
        assert_eq!(rendered.html, None);
    }

    #[test]
    fn lists_render_as_html_lists() {
        let rendered = render("Groceries:\n1. Milk\n2. Eggs\n\n- a & b");
        assert_eq!(rendered.plain, "Groceries:\n1. Milk\n2. Eggs\n\n- a & b");
        assert_eq!(
            rendered.html.unwrap(),
            "<meta charset=\"utf-8\">Groceries:<ol><li>Milk</li><li>Eggs</li></ol><ul><li>a &amp; b</li></ul>"
        );
    }

    #[test]
    fn snippet_markup_renders_bold_italic_and_links() {
        let text = format!(
            "Thanks! {}",
            snippet("**Dan** _Shrimp_ [cal](https://cal.com/dan) \\*literal\\*")
        );
        let rendered = render(&text);
        assert_eq!(
            rendered.plain,
            "Thanks! Dan Shrimp cal (https://cal.com/dan) *literal*"
        );
        assert_eq!(
            rendered.html.unwrap(),
            "<meta charset=\"utf-8\">Thanks! <b>Dan</b> <i>Shrimp</i> <a href=\"https://cal.com/dan\">cal</a> *literal*"
        );
    }

    #[test]
    fn paragraphs_are_separated_by_breaks() {
        let text = format!("{}\n\nSecond", snippet("**One**\nline"));
        assert_eq!(
            render(&text).html.unwrap(),
            "<meta charset=\"utf-8\"><b>One</b><br>line<br><br>Second"
        );
    }

    #[test]
    fn unsafe_links_stay_text() {
        let rendered = render(&snippet("[x](javascript:alert(1))"));
        assert_eq!(rendered.plain, "[x](javascript:alert(1))");
    }

    #[test]
    fn escape_round_trips_through_plain() {
        let original = "a *b* [c] d_e \\";
        assert_eq!(
            plain(&snippet(&crate::storage::snippets::escape_markdown(
                original
            ))),
            original
        );
    }
}
