#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag {
    Bold,
    Italic,
    Underline,
    Strike,
    Colour,
    Size,
    Font,
    Superscript,
    Subscript,
    Align(Alignment),
    Spoiler,
    Url,
    Email,
    Image,
    YouTube,
    List,
    Item,
    Quote,
    Indent,
    Code,
    Highlight,
    Rule,
    Heading,
    Table,
    Row,
    HeaderCell,
    Cell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

impl Alignment {
    fn css(self) -> &'static str {
        match self {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "justify",
        }
    }
}

impl Tag {
    fn is_text_style(self) -> bool {
        matches!(
            self,
            Tag::Bold
                | Tag::Italic
                | Tag::Underline
                | Tag::Strike
                | Tag::Colour
                | Tag::Size
                | Tag::Font
                | Tag::Superscript
                | Tag::Subscript
        )
    }

    fn is_block(self) -> bool {
        matches!(
            self,
            Tag::Align(_)
                | Tag::Spoiler
                | Tag::List
                | Tag::Item
                | Tag::Quote
                | Tag::Indent
                | Tag::Code
                | Tag::Rule
                | Tag::Heading
                | Tag::Table
                | Tag::Row
                | Tag::HeaderCell
                | Tag::Cell
        )
    }

    fn from_name(name: &str) -> Option<(Tag, &'static str)> {
        let tag = match name.to_ascii_lowercase().as_str() {
            "b" => (Tag::Bold, "b"),
            "i" => (Tag::Italic, "i"),
            "u" => (Tag::Underline, "u"),
            "s" | "strike" => (Tag::Strike, "s"),
            "color" => (Tag::Colour, "color"),
            "size" => (Tag::Size, "size"),
            "font" => (Tag::Font, "font"),
            "sup" => (Tag::Superscript, "sup"),
            "sub" => (Tag::Subscript, "sub"),
            "left" => (Tag::Align(Alignment::Left), "left"),
            "center" => (Tag::Align(Alignment::Center), "center"),
            "right" => (Tag::Align(Alignment::Right), "right"),
            "justify" => (Tag::Align(Alignment::Justify), "justify"),
            "spoiler" => (Tag::Spoiler, "spoiler"),
            "url" => (Tag::Url, "url"),
            "email" => (Tag::Email, "email"),
            "img" => (Tag::Image, "img"),
            "img_left" => (Tag::Image, "img_left"),
            "img_right" => (Tag::Image, "img_right"),
            "youtube" => (Tag::YouTube, "youtube"),
            "list" => (Tag::List, "list"),
            "*" => (Tag::Item, "*"),
            "quote" => (Tag::Quote, "quote"),
            "indent" => (Tag::Indent, "indent"),
            "code" | "php" | "html" | "noparse" => (Tag::Code, "code"),
            "highlight" => (Tag::Highlight, "highlight"),
            "hr" => (Tag::Rule, "hr"),
            "heading" => (Tag::Heading, "heading"),
            "table" => (Tag::Table, "table"),
            "tr" => (Tag::Row, "tr"),
            "th" => (Tag::HeaderCell, "th"),
            "td" => (Tag::Cell, "td"),
            _ => return None,
        };

        Some(tag)
    }
}

#[derive(Debug)]
enum Node {
    Text(String),
    Element {
        tag: Tag,
        argument: Option<String>,
        children: Vec<Node>,
    },
}

struct Open {
    tag: Tag,
    name: &'static str,
    argument: Option<String>,
    children: Vec<Node>,
}

enum Token<'a> {
    Open {
        tag: Tag,
        name: &'static str,
        argument: Option<String>,
        raw_name: &'a str,
    },
    Close {
        name: &'static str,
    },
}

pub fn to_html(bbcode: &str) -> String {
    let normalised = bbcode.replace("\r\n", "\n").replace('\r', "\n");

    render(&parse(&normalised), false)
}

pub fn urls(bbcode: &str) -> Vec<String> {
    let normalised = bbcode.replace("\r\n", "\n").replace('\r', "\n");
    let mut urls = Vec::new();
    collect_urls(&parse(&normalised), &mut urls);
    urls
}

fn collect_urls(nodes: &[Node], urls: &mut Vec<String>) {
    for node in nodes {
        let Node::Element {
            tag,
            argument,
            children,
        } = node
        else {
            continue;
        };
        if *tag == Tag::Url {
            let target = argument.clone().unwrap_or_else(|| plain_text(children));
            let target = target.trim();
            if !target.is_empty() {
                urls.push(target.to_owned());
            }
        }
        collect_urls(children, urls);
    }
}

fn parse(input: &str) -> Vec<Node> {
    let mut root = Vec::new();
    let mut stack: Vec<Open> = Vec::new();
    let lowercase = input.to_ascii_lowercase();
    let mut rest = input;
    let mut line_start = true;

    while !rest.is_empty() {
        let Some(start) = rest.find('[') else {
            push_source(&mut root, &mut stack, rest, &mut line_start);
            break;
        };

        push_source(&mut root, &mut stack, &rest[..start], &mut line_start);
        rest = &rest[start..];
        line_start = false;

        let Some((token, length)) = token(rest) else {
            push_text(current(&mut root, &mut stack), "[");
            rest = &rest[1..];
            continue;
        };

        let raw = &rest[..length];
        rest = &rest[length..];
        let following = &lowercase[input.len() - rest.len()..];

        if let Token::Open {
            tag,
            name,
            raw_name,
            ..
        } = &token
            && !matches!(tag, Tag::Item | Tag::Rule)
            && !has_closing_tag(following, name)
            && !has_closing_tag(following, &raw_name.to_ascii_lowercase())
        {
            push_text(current(&mut root, &mut stack), raw);
            continue;
        }

        match token {
            Token::Open {
                tag: Tag::Code,
                argument,
                raw_name,
                ..
            } => {
                let (code, remaining) = take_raw(rest, raw_name);
                current(&mut root, &mut stack).push(Node::Element {
                    tag: Tag::Code,
                    argument,
                    children: vec![Node::Text(code.to_owned())],
                });
                rest = remaining;
            }
            Token::Open {
                tag: Tag::Highlight,
                argument: Some(language),
                raw_name,
                ..
            } if !language.trim().is_empty() => {
                let (code, remaining) = take_raw(rest, raw_name);
                current(&mut root, &mut stack).push(Node::Element {
                    tag: Tag::Code,
                    argument: Some(language),
                    children: vec![Node::Text(code.to_owned())],
                });
                rest = remaining;
            }
            Token::Open { tag: Tag::Rule, .. } => {
                current(&mut root, &mut stack).push(Node::Element {
                    tag: Tag::Rule,
                    argument: None,
                    children: Vec::new(),
                })
            }
            Token::Open {
                tag: Tag::Item,
                argument,
                ..
            } => open_item(&mut root, &mut stack, argument),
            Token::Open {
                tag,
                name,
                argument,
                ..
            } => stack.push(Open {
                tag,
                name,
                argument,
                children: Vec::new(),
            }),
            Token::Close { name } => {
                if let Some(position) = stack.iter().rposition(|open| open.name == name) {
                    let reopened: Vec<(Tag, &'static str, Option<String>)> = stack[position + 1..]
                        .iter()
                        .filter(|open| {
                            open.tag.is_text_style() && has_closing_tag(following, open.name)
                        })
                        .map(|open| (open.tag, open.name, open.argument.clone()))
                        .collect();

                    while stack.len() > position {
                        close_top(&mut root, &mut stack);
                    }

                    stack.extend(reopened.into_iter().map(|(tag, name, argument)| Open {
                        tag,
                        name,
                        argument,
                        children: Vec::new(),
                    }));
                }
            }
        }
    }

    while !stack.is_empty() {
        close_top(&mut root, &mut stack);
    }

    root
}

fn push_source(root: &mut Vec<Node>, stack: &mut Vec<Open>, text: &str, line_start: &mut bool) {
    let mut rest = text;

    while !rest.is_empty() {
        if *line_start
            && in_list(stack)
            && let Some(after) = rest.trim_start_matches([' ', '\t']).strip_prefix('*')
        {
            open_item(root, stack, None);
            rest = after;
            *line_start = false;
            continue;
        }

        let line_end = rest.find('\n').map_or(rest.len(), |position| position + 1);
        push_text(current(root, stack), &rest[..line_end]);
        *line_start = rest[..line_end].ends_with('\n');
        rest = &rest[line_end..];
    }
}

fn in_list(stack: &[Open]) -> bool {
    match stack {
        [.., last] if last.tag == Tag::List => true,
        [.., parent, last] => last.tag == Tag::Item && parent.tag == Tag::List,
        _ => false,
    }
}

fn open_item(root: &mut Vec<Node>, stack: &mut Vec<Open>, argument: Option<String>) {
    if stack.last().is_some_and(|open| open.tag == Tag::Item) {
        close_top(root, stack);
    }

    stack.push(Open {
        tag: Tag::Item,
        name: "*",
        argument,
        children: Vec::new(),
    });
}

fn has_closing_tag(lowercase: &str, name: &str) -> bool {
    lowercase.contains(&format!("[/{name}]"))
}

fn token(input: &str) -> Option<(Token<'_>, usize)> {
    let end = input.find(']')?;
    let inner = &input[1..end];
    let length = end + 1;

    if let Some(name) = inner.strip_prefix('/') {
        let (_, name) = Tag::from_name(name.trim())?;
        return Some((Token::Close { name }, length));
    }

    let (raw_name, argument) = match inner.split_once('=') {
        Some((name, argument)) => (name, Some(unquote(argument))),
        None => (inner, None),
    };
    let (tag, name) = Tag::from_name(raw_name.trim())?;

    Some((
        Token::Open {
            tag,
            name,
            argument,
            raw_name: raw_name.trim(),
        },
        length,
    ))
}

fn unquote(argument: &str) -> String {
    let argument = argument.trim();

    argument
        .strip_prefix('"')
        .and_then(|argument| argument.strip_suffix('"'))
        .or_else(|| {
            argument
                .strip_prefix('\'')
                .and_then(|argument| argument.strip_suffix('\''))
        })
        .unwrap_or(argument)
        .to_owned()
}

fn take_raw<'a>(input: &'a str, name: &str) -> (&'a str, &'a str) {
    let closing = format!("[/{}]", name.to_ascii_lowercase());
    let lowercase = input.to_ascii_lowercase();

    match lowercase.find(&closing) {
        Some(position) => (&input[..position], &input[position + closing.len()..]),
        None => (input, ""),
    }
}

fn current<'a>(root: &'a mut Vec<Node>, stack: &'a mut [Open]) -> &'a mut Vec<Node> {
    match stack.last_mut() {
        Some(open) => &mut open.children,
        None => root,
    }
}

fn push_text(nodes: &mut Vec<Node>, text: &str) {
    if text.is_empty() {
        return;
    }

    if let Some(Node::Text(previous)) = nodes.last_mut() {
        previous.push_str(text);
    } else {
        nodes.push(Node::Text(text.to_owned()));
    }
}

fn close_top(root: &mut Vec<Node>, stack: &mut Vec<Open>) {
    let Some(open) = stack.pop() else {
        return;
    };

    current(root, stack).push(Node::Element {
        tag: open.tag,
        argument: open.argument,
        children: open.children,
    });
}

fn render(nodes: &[Node], in_link: bool) -> String {
    let mut output = String::new();

    for (index, node) in nodes.iter().enumerate() {
        match node {
            Node::Text(text) => {
                let mut text = text.as_str();
                if index > 0 && is_block_node(&nodes[index - 1]) {
                    text = text.strip_prefix('\n').unwrap_or(text);
                }
                if nodes.get(index + 1).is_some_and(is_block_node) {
                    text = text.strip_suffix('\n').unwrap_or(text);
                }
                output.push_str(&render_text(text, in_link));
            }
            Node::Element {
                tag,
                argument,
                children,
            } => output.push_str(&render_element(
                *tag,
                argument.as_deref(),
                children,
                in_link,
            )),
        }
    }

    output
}

fn is_block_node(node: &Node) -> bool {
    matches!(node, Node::Element { tag, .. } if tag.is_block())
}

fn render_element(tag: Tag, argument: Option<&str>, children: &[Node], in_link: bool) -> String {
    let inner = || render(children, in_link);

    match tag {
        Tag::Bold => format!("<b>{}</b>", inner()),
        Tag::Italic => format!("<i>{}</i>", inner()),
        Tag::Underline => format!("<u>{}</u>", inner()),
        Tag::Strike => format!("<s>{}</s>", inner()),
        Tag::Superscript => format!("<sup>{}</sup>", inner()),
        Tag::Subscript => format!("<sub>{}</sub>", inner()),
        Tag::Font => inner(),
        Tag::Colour => match argument.and_then(css_colour) {
            Some(colour) => format!("<span style=\"color:{colour}\">{}</span>", inner()),
            None => inner(),
        },
        Tag::Size => match argument.and_then(font_size) {
            Some(size) => format!("<span style=\"font-size:{size}rem\">{}</span>", inner()),
            None => inner(),
        },
        Tag::Align(alignment) => format!(
            "<div style=\"text-align:{}\">{}</div>",
            alignment.css(),
            inner()
        ),
        Tag::Spoiler => format!("<div class=\"spoiler\">{}</div>", inner()),
        Tag::Url => {
            let target = argument
                .map(str::to_owned)
                .unwrap_or_else(|| plain_text(children));
            let label = if argument.is_some() {
                render(children, true)
            } else {
                escape(target.trim())
            };

            link(&label, &with_scheme(&target), in_link)
        }
        Tag::Email => {
            let address = argument
                .map(str::to_owned)
                .unwrap_or_else(|| plain_text(children));
            let label = if argument.is_some() {
                render(children, true)
            } else {
                escape(address.trim())
            };

            link(&label, &format!("mailto:{}", address.trim()), in_link)
        }
        Tag::Image => {
            let source = with_scheme(&plain_text(children));

            if source.is_empty() {
                String::new()
            } else {
                format!("<img src=\"{}\">", attribute(&source))
            }
        }
        Tag::YouTube => {
            let id: String = plain_text(children)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                .collect();
            let preview =
                format!("<img src=\"https://img.youtube.com/vi/{id}/mqdefault.jpg\"><br>YouTube");

            link(
                &preview,
                &format!("https://www.youtube.com/watch?v={id}"),
                in_link,
            )
        }
        Tag::List => render_list(argument, children, in_link),
        Tag::Item => format!("<ul><li>{}</li></ul>", inner()),
        Tag::Quote => {
            let author = argument
                .map(str::trim)
                .filter(|author| !author.is_empty())
                .map(|author| format!("<div class=\"quote-author\">{}:</div>", escape(author)))
                .unwrap_or_default();

            format!("<blockquote>{author}{}</blockquote>", inner())
        }
        Tag::Indent => format!("<div class=\"indent\">{}</div>", inner()),
        Tag::Code => format!(
            "<pre>{}</pre>",
            escape(plain_text(children).trim_matches('\n'))
        ),
        Tag::Highlight => format!("<code>{}</code>", inner()),
        Tag::Rule => "<hr>".to_owned(),
        Tag::Heading => {
            let level = argument
                .and_then(|level| level.trim().parse::<usize>().ok())
                .unwrap_or(3)
                .clamp(1, 6);

            format!("<h{level}>{}</h{level}>", inner())
        }
        Tag::Table => format!(
            "<table>{}</table>",
            render_only(children, &[Tag::Row], in_link)
        ),
        Tag::Row => format!(
            "<tr>{}</tr>",
            render_only(children, &[Tag::HeaderCell, Tag::Cell], in_link)
        ),
        Tag::HeaderCell => format!("<th>{}</th>", inner()),
        Tag::Cell => format!("<td>{}</td>", inner()),
    }
}

fn render_only(children: &[Node], tags: &[Tag], in_link: bool) -> String {
    children
        .iter()
        .filter_map(|child| match child {
            Node::Element {
                tag,
                argument,
                children,
            } if tags.contains(tag) => {
                Some(render_element(*tag, argument.as_deref(), children, in_link))
            }
            _ => None,
        })
        .collect()
}

fn render_list(argument: Option<&str>, children: &[Node], in_link: bool) -> String {
    let kind = argument.map(str::trim).unwrap_or_default();
    let (open, close) = match kind {
        "" => ("<ul>".to_owned(), "</ul>"),
        "a" | "A" | "i" | "I" => (format!("<ol type=\"{kind}\">"), "</ol>"),
        _ => ("<ol>".to_owned(), "</ol>"),
    };

    let items: String = children
        .iter()
        .map(|child| match child {
            Node::Element {
                tag: Tag::Item,
                children,
                ..
            } => format!("<li>{}</li>", render(children, in_link)),
            other => {
                let content = render(std::slice::from_ref(other), in_link);
                if content.replace("<br>", "").trim().is_empty() {
                    String::new()
                } else {
                    format!("<li class=\"text\">{content}</li>")
                }
            }
        })
        .collect();

    format!("{open}{items}{close}")
}

fn plain_text(nodes: &[Node]) -> String {
    let mut output = String::new();

    for node in nodes {
        match node {
            Node::Text(text) => output.push_str(text),
            Node::Element { children, .. } => output.push_str(&plain_text(children)),
        }
    }

    output
}

fn with_scheme(target: &str) -> String {
    let target = target.trim();

    if target.contains("://") || target.starts_with("mailto:") || target.is_empty() {
        target.to_owned()
    } else {
        format!("http://{target}")
    }
}

fn link(label: &str, target: &str, in_link: bool) -> String {
    let label = if label.trim().is_empty() {
        escape(target)
    } else {
        label.trim().to_owned()
    };

    if in_link || target.is_empty() {
        return label;
    }

    format!("<a href=\"{}\">{label}</a>", attribute(target))
}

fn css_colour(value: &str) -> Option<String> {
    let value = value.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);

    if matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{hex}"))
    } else if !value.is_empty() && value.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(value.to_ascii_lowercase())
    } else {
        None
    }
}

fn font_size(value: &str) -> Option<f32> {
    let size = match value.trim().parse::<u32>().ok()? {
        0 | 1 => 0.625,
        2 => 0.8125,
        3 => 1.0,
        4 => 1.125,
        5 => 1.5,
        6 => 2.0,
        _ => 3.0,
    };

    Some(size)
}

fn render_text(text: &str, in_link: bool) -> String {
    let mut output = String::new();
    let mut rest = text;

    while !rest.is_empty() {
        let next_url = if in_link {
            None
        } else {
            ["https://", "http://"]
                .iter()
                .filter_map(|scheme| rest.find(scheme))
                .min()
        };

        let Some(start) = next_url else {
            output.push_str(&escape_with_breaks(rest));
            break;
        };

        output.push_str(&escape_with_breaks(&rest[..start]));

        let url_length = rest[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"'))
            .unwrap_or(rest.len() - start);
        let url =
            rest[start..start + url_length].trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);

        output.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            attribute(url),
            escape(url)
        ));
        rest = &rest[start + url.len()..];
    }

    output
}

fn escape_with_breaks(text: &str) -> String {
    escape(text).replace('\n', "<br>\n")
}

fn escape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            c => output.push(c),
        }
    }

    output
}

fn attribute(value: &str) -> String {
    escape(
        &value
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>(),
    )
}

#[cfg(test)]
mod tests {
    use super::to_html;

    #[test]
    fn converts_text_styles() {
        assert_eq!(
            to_html("[b]b[/b] [i]i[/i] [u]u[/u] [s]s[/s] [strike]x[/strike] [sup]1[/sup]"),
            "<b>b</b> <i>i</i> <u>u</u> <s>s</s> <s>x</s> <sup>1</sup>"
        );
    }

    #[test]
    fn converts_colours_and_sizes() {
        assert_eq!(
            to_html(
                "[COLOR=\"DarkOrange\"]a[/COLOR][color=#FF0000]b[/color][color=E5E5E5]c[/color]"
            ),
            "<span style=\"color:darkorange\">a</span><span style=\"color:#FF0000\">b</span><span style=\"color:#E5E5E5\">c</span>"
        );
        assert_eq!(
            to_html("[color=red;background:x]a[/color] [size=5]b[/size] [size=big]c[/size]"),
            "a <span style=\"font-size:1.5rem\">b</span> c"
        );
        assert_eq!(to_html("[font=Georgia]a[/font]"), "a");
    }

    #[test]
    fn converts_alignment_and_spoilers() {
        assert_eq!(
            to_html("[CENTER]a[/CENTER][right]b[/right]"),
            "<div style=\"text-align:center\">a</div><div style=\"text-align:right\">b</div>"
        );
        assert_eq!(
            to_html("[spoiler]x[/spoiler]"),
            "<div class=\"spoiler\">x</div>"
        );
    }

    #[test]
    fn escapes_html_and_keeps_line_breaks() {
        assert_eq!(
            to_html("a < b & \"c\"\r\nnext"),
            "a &lt; b &amp; &quot;c&quot;<br>\nnext"
        );
    }

    #[test]
    fn converts_links() {
        assert_eq!(
            to_html("[URL=\"http://site.com/\"]site[/URL]"),
            "<a href=\"http://site.com/\">site</a>"
        );
        assert_eq!(
            to_html("[url]www.esoui.com[/url]"),
            "<a href=\"http://www.esoui.com\">www.esoui.com</a>"
        );
        assert_eq!(
            to_html("[email]a@b.c[/email]"),
            "<a href=\"mailto:a@b.c\">a@b.c</a>"
        );
        assert_eq!(
            to_html("See https://www.esoui.com/x. Thanks"),
            "See <a href=\"https://www.esoui.com/x\">https://www.esoui.com/x</a>. Thanks"
        );
    }

    #[test]
    fn converts_images_inside_and_outside_links() {
        assert_eq!(
            to_html("[img]x.com/a.png[/img]"),
            "<img src=\"http://x.com/a.png\">"
        );
        assert_eq!(
            to_html("[url=https://site.com][img]https://x.com/a.png[/img][/url]"),
            "<a href=\"https://site.com\"><img src=\"https://x.com/a.png\"></a>"
        );
        assert_eq!(
            to_html("[img_right]https://x.com/a.png[/img_right] text"),
            "<img src=\"https://x.com/a.png\"> text"
        );
    }

    #[test]
    fn converts_youtube_to_a_linked_preview() {
        assert_eq!(
            to_html("[youtube]givgWDJHIjI[/youtube]"),
            "<a href=\"https://www.youtube.com/watch?v=givgWDJHIjI\"><img src=\"https://img.youtube.com/vi/givgWDJHIjI/mqdefault.jpg\"><br>YouTube</a>"
        );
    }

    #[test]
    fn converts_lists() {
        assert_eq!(
            to_html("[list]\n[*]one\n[*]two\n[/list]\nafter"),
            "<ul><li>one<br>\n</li><li>two<br>\n</li></ul>after"
        );
        assert_eq!(to_html("[LIST=1][*]a[/LIST]"), "<ol><li>a</li></ol>");
        assert_eq!(
            to_html("[list=a][*]a[/list]"),
            "<ol type=\"a\"><li>a</li></ol>"
        );
        assert_eq!(
            to_html("[list][*]a[list][*]b[/list][/list]"),
            "<ul><li>a<ul><li>b</li></ul></li></ul>"
        );
    }

    #[test]
    fn treats_bare_stars_in_lists_as_items() {
        assert_eq!(
            to_html("[LIST]\n*one\n[*]two[/LIST]"),
            "<ul><li>one<br>\n</li><li>two</li></ul>"
        );
    }

    #[test]
    fn converts_quotes_and_indents() {
        assert_eq!(
            to_html("[quote=Bob]hi[/quote]"),
            "<blockquote><div class=\"quote-author\">Bob:</div>hi</blockquote>"
        );
        assert_eq!(
            to_html("[indent]a[/indent]"),
            "<div class=\"indent\">a</div>"
        );
    }

    #[test]
    fn keeps_code_raw() {
        assert_eq!(
            to_html("[code]\nif a < [b]x[/b]\n    y()\n[/code]"),
            "<pre>if a &lt; [b]x[/b]\n    y()</pre>"
        );
        assert_eq!(
            to_html("[highlight=\"Lua\"]local x = 1[/highlight]"),
            "<pre>local x = 1</pre>"
        );
        assert_eq!(
            to_html("Type [highlight]/goblind[/highlight]"),
            "Type <code>/goblind</code>"
        );
    }

    #[test]
    fn converts_rules_headings_and_tables() {
        assert_eq!(to_html("a[hr]b"), "a<hr>b");
        assert_eq!(to_html("[HEADING=2]Links[/HEADING]"), "<h2>Links</h2>");
        assert_eq!(
            to_html("[table]\n[tr][th]A[/th][/tr]\n[tr][td]1[/td][/tr]\n[/table]"),
            "<table><tr><th>A</th></tr><tr><td>1</td></tr></table>"
        );
    }

    #[test]
    fn reopens_text_styles_closed_in_the_wrong_order() {
        assert_eq!(
            to_html("[color=yellow][b]Set[/color]![/b]"),
            "<span style=\"color:yellow\"><b>Set</b></span><b>!</b>"
        );
    }

    #[test]
    fn keeps_tags_without_closing_tag_as_text() {
        assert_eq!(to_html("[b]bold"), "[b]bold");
        assert_eq!(to_html("verse[s]/vision[s]"), "verse[s]/vision[s]");
        assert_eq!(to_html("[WIP] done[/b]"), "[WIP] done");
    }
}
