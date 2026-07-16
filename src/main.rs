use clap::Parser;
use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser as MdParser, Tag, TagEnd,
};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

#[derive(Parser)]
#[command(name = "cat", version, about = "cat, but with syntax highlighting and markdown preview")]
struct Cli {
    #[arg(short = 'A', long = "show-all")]
    show_all: bool,
    #[arg(short = 'b', long = "number-nonblank")]
    number_nonblank: bool,
    #[arg(short = 'e')]
    show_ends_v: bool,
    #[arg(short = 'E', long = "show-ends")]
    show_ends: bool,
    #[arg(short = 'n', long = "number")]
    number: bool,
    #[arg(short = 's', long = "squeeze-blank")]
    squeeze_blank: bool,
    #[arg(short = 'T', long = "show-tabs")]
    show_tabs: bool,
    #[arg(short = 'v', long = "show-nonprinting")]
    show_nonprinting: bool,
    #[arg(short = 'p', long = "plain")]
    plain: bool,
    #[arg(long = "syntax-highlighting", default_value = "on")]
    syntax_highlighting: String,
    #[arg(long = "markdown-preview", default_value = "on")]
    markdown_preview: String,
    #[arg(short = 'l', long = "language")]
    language: Option<String>,
    #[arg(long = "theme", default_value = "ocean-dark")]
    theme: String,
    #[arg(short = 'r', long = "range")]
    range: Option<String>,
    files: Vec<String>,
}

fn read_input(path: &str) -> io::Result<String> {
    let bytes = if path == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        fs::read(path)?
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn resolve_theme(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "ocean-dark" | "ocean" | "dark" => "base16-ocean.dark",
        "ocean-light" | "light" => "base16-ocean.light",
        "eighties" => "base16-eighties.dark",
        "mocha" => "base16-mocha.dark",
        "github" => "InspiredGitHub",
        "solarized-dark" => "Solarized (dark)",
        "solarized-light" => "Solarized (light)",
        _ => "base16-ocean.dark",
    }
}

fn pick_theme<'a>(ts: &'a ThemeSet, key: &str) -> &'a Theme {
    ts.themes
        .get(key)
        .unwrap_or_else(|| &ts.themes["base16-ocean.dark"])
}

fn parse_range(spec: &str, total: usize) -> (usize, usize) {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    let mut start = parts
        .first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let mut end = parts
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(total);
    if start < 1 {
        start = 1;
    }
    if end > total {
        end = total;
    }
    (start, end)
}

fn apply_range(content: &str, spec: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let total = lines.len();
    if total == 0 {
        return String::new();
    }
    let (start, end) = parse_range(spec, total);
    if start > end {
        return String::new();
    }
    let mut s = lines[start - 1..end].join("\n");
    s.push('\n');
    s
}

fn print_raw(content: &str, cli: &Cli) {
    let show_tabs = cli.show_tabs || cli.show_all;
    let show_ends = cli.show_ends || cli.show_all || cli.show_ends_v;
    let show_nonprinting = cli.show_nonprinting || cli.show_all || cli.show_ends_v;
    let number_all = cli.number;
    let number_nonblank = cli.number_nonblank;
    let squeeze = cli.squeeze_blank;

    let ends_with_newline = content.ends_with('\n');
    let mut lines: Vec<&str> = content.split('\n').collect();
    if ends_with_newline {
        lines.pop();
    }

    let mut line_no: u64 = 0;
    let mut prev_blank = false;
    let mut out = String::with_capacity(content.len() + content.len() / 8);

    for line in lines {
        let is_blank = line.is_empty();
        if squeeze && is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;

        if number_nonblank {
            if !is_blank {
                line_no += 1;
                out.push_str(&format!("{:>6}\t", line_no));
            }
        } else if number_all {
            line_no += 1;
            out.push_str(&format!("{:>6}\t", line_no));
        }

        for ch in line.chars() {
            let code = ch as u32;
            if ch == '\t' {
                if show_tabs {
                    out.push_str("^I");
                } else {
                    out.push('\t');
                }
            } else if code < 32 {
                if show_nonprinting {
                    out.push('^');
                    out.push((code as u8 + 64) as char);
                } else {
                    out.push(ch);
                }
            } else if code == 127 {
                if show_nonprinting {
                    out.push_str("^?");
                } else {
                    out.push(ch);
                }
            } else if (128..256).contains(&code) {
                if show_nonprinting {
                    let low = code - 128;
                    out.push_str("M-");
                    if low < 32 {
                        out.push('^');
                        out.push((low as u8 + 64) as char);
                    } else if low == 127 {
                        out.push_str("^?");
                    } else {
                        out.push(low as u8 as char);
                    }
                } else {
                    out.push(ch);
                }
            } else {
                out.push(ch);
            }
        }

        if show_ends {
            out.push('$');
        }
        out.push('\n');
    }

    let _ = io::stdout().write_all(out.as_bytes());
}

fn find_syntax<'a>(
    ss: &'a SyntaxSet,
    filename: &str,
    content: &str,
    forced: &Option<String>,
) -> &'a SyntaxReference {
    if let Some(lang) = forced {
        if let Some(s) = ss.find_syntax_by_token(lang) {
            return s;
        }
        if let Some(s) = ss.find_syntax_by_extension(lang) {
            return s;
        }
    }
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.is_empty() {
        if let Some(s) = ss.find_syntax_by_extension(ext) {
            return s;
        }
    }
    if let Some(first_line) = content.lines().next() {
        if let Some(s) = ss.find_syntax_by_first_line(first_line) {
            return s;
        }
    }
    ss.find_syntax_plain_text()
}

fn highlight_and_print(content: &str, filename: &str, theme_key: &str, forced_lang: &Option<String>) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = find_syntax(&ss, filename, content, forced_lang);
    let theme = pick_theme(&ts, theme_key);
    let mut h = HighlightLines::new(syntax, theme);
    let mut out = String::with_capacity(content.len() * 2);

    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, &ss).unwrap_or_default();
        out.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
    }
    out.push_str("\x1b[0m");
    let _ = io::stdout().write_all(out.as_bytes());
}

fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

fn pad_cell(s: &str, width: usize, align: &Alignment) -> String {
    let vw = visible_width(s);
    let diff = width.saturating_sub(vw);
    match align {
        Alignment::Right => format!("{}{}", " ".repeat(diff), s),
        Alignment::Center => {
            let left = diff / 2;
            let right = diff - left;
            format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
        }
        _ => format!("{}{}", s, " ".repeat(diff)),
    }
}

fn table_border(widths: &[usize], left: &str, mid: &str, right: &str) -> String {
    let mut s = String::from(left);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&"─".repeat(w + 2));
        if i < widths.len() - 1 {
            s.push_str(mid);
        }
    }
    s.push_str(right);
    s
}

fn print_table(header: &[String], rows: &[Vec<String>], alignments: &[Alignment]) {
    if header.is_empty() && rows.is_empty() {
        return;
    }
    let row_max = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let col_count = header.len().max(row_max).max(alignments.len());
    if col_count == 0 {
        return;
    }

    let mut widths = vec![3usize; col_count];
    for (i, h) in header.iter().enumerate() {
        if i < col_count {
            widths[i] = widths[i].max(visible_width(h));
        }
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(visible_width(cell));
            }
        }
    }

    println!("\x1b[2m{}\x1b[0m", table_border(&widths, "┌", "┬", "┐"));

    let mut header_line = String::from("\x1b[2m│\x1b[0m ");
    for i in 0..col_count {
        let cell = header.get(i).map(|s| s.as_str()).unwrap_or("");
        let align = alignments.get(i).unwrap_or(&Alignment::None);
        header_line.push_str(&format!("\x1b[1;36m{}\x1b[0m", pad_cell(cell, widths[i], align)));
        header_line.push_str(" \x1b[2m│\x1b[0m ");
    }
    println!("{}", header_line.trim_end());

    println!("\x1b[2m{}\x1b[0m", table_border(&widths, "├", "┼", "┤"));

    for row in rows {
        let mut line = String::from("\x1b[2m│\x1b[0m ");
        for i in 0..col_count {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let align = alignments.get(i).unwrap_or(&Alignment::None);
            line.push_str(&pad_cell(cell, widths[i], align));
            line.push_str(" \x1b[2m│\x1b[0m ");
        }
        println!("{}", line.trim_end());
    }

    println!("\x1b[2m{}\x1b[0m", table_border(&widths, "└", "┴", "┘"));
    println!();
}

fn render_markdown(content: &str, theme_key: &str) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = pick_theme(&ts, theme_key);

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_SMART_PUNCTUATION;
    let parser = MdParser::new_ext(content, options);

    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buffer = String::new();

    let mut in_table = false;
    let mut table_alignments: Vec<Alignment> = Vec::new();
    let mut header_row: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut cell_buffer = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let prefix = match level {
                    HeadingLevel::H1 => "\n\x1b[1;4;95m# ",
                    HeadingLevel::H2 => "\n\x1b[1;95m## ",
                    HeadingLevel::H3 => "\n\x1b[1;36m### ",
                    _ => "\n\x1b[1;36m#### ",
                };
                print!("{}", prefix);
            }
            Event::End(TagEnd::Heading(_)) => {
                println!("\x1b[0m");
            }
            Event::Start(Tag::Strong) => {
                if in_table {
                    cell_buffer.push_str("\x1b[1m");
                } else {
                    print!("\x1b[1m");
                }
            }
            Event::End(TagEnd::Strong) => {
                if in_table {
                    cell_buffer.push_str("\x1b[22m");
                } else {
                    print!("\x1b[22m");
                }
            }
            Event::Start(Tag::Emphasis) => {
                if in_table {
                    cell_buffer.push_str("\x1b[3m");
                } else {
                    print!("\x1b[3m");
                }
            }
            Event::End(TagEnd::Emphasis) => {
                if in_table {
                    cell_buffer.push_str("\x1b[23m");
                } else {
                    print!("\x1b[23m");
                }
            }
            Event::Start(Tag::Strikethrough) => {
                if in_table {
                    cell_buffer.push_str("\x1b[9m");
                } else {
                    print!("\x1b[9m");
                }
            }
            Event::End(TagEnd::Strikethrough) => {
                if in_table {
                    cell_buffer.push_str("\x1b[29m");
                } else {
                    print!("\x1b[29m");
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_buffer.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                println!("\x1b[2m┌───────────────────────────────────────\x1b[0m");
            }
            Event::End(TagEnd::CodeBlock) => {
                let syntax = if !code_lang.is_empty() {
                    ss.find_syntax_by_token(&code_lang)
                        .unwrap_or_else(|| ss.find_syntax_plain_text())
                } else {
                    ss.find_syntax_plain_text()
                };
                let mut h = HighlightLines::new(syntax, theme);
                for line in LinesWithEndings::from(&code_buffer) {
                    let ranges = h.highlight_line(line, &ss).unwrap_or_default();
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    print!("  {}", escaped);
                }
                print!("\x1b[0m");
                println!("\x1b[2m└───────────────────────────────────────\x1b[0m");
                in_code_block = false;
            }
            Event::Code(text) => {
                if in_table {
                    cell_buffer.push_str(&format!("\x1b[38;5;222m{}\x1b[0m", text));
                } else {
                    print!("\x1b[38;5;222m{}\x1b[0m", text);
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else if in_table {
                    cell_buffer.push_str(&text);
                } else {
                    print!("{}", text);
                }
            }
            Event::Start(Tag::List(start)) => {
                list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                let indent = "  ".repeat(depth);
                match list_stack.last_mut() {
                    Some(Some(n)) => {
                        print!("\n{}\x1b[36m{}.\x1b[0m ", indent, n);
                        *n += 1;
                    }
                    _ => {
                        print!("\n{}\x1b[36m•\x1b[0m ", indent);
                    }
                }
            }
            Event::End(TagEnd::Item) => {}
            Event::TaskListMarker(checked) => {
                if checked {
                    print!("\x1b[32m[x]\x1b[0m ");
                } else {
                    print!("\x1b[2m[ ]\x1b[0m ");
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                print!("\x1b[2;3m▏ ");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                println!("\x1b[0m");
            }
            Event::Start(Tag::Link { .. }) => {
                if in_table {
                    cell_buffer.push_str("\x1b[4;34m");
                } else {
                    print!("\x1b[4;34m");
                }
            }
            Event::End(TagEnd::Link) => {
                if in_table {
                    cell_buffer.push_str("\x1b[0m");
                } else {
                    print!("\x1b[0m");
                }
            }
            Event::Start(Tag::Table(alignments)) => {
                in_table = true;
                table_alignments = alignments;
                header_row.clear();
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                print_table(&header_row, &table_rows, &table_alignments);
                in_table = false;
            }
            Event::Start(Tag::TableHead) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                header_row = std::mem::take(&mut current_row);
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push(std::mem::take(&mut current_row));
            }
            Event::Start(Tag::TableCell) => {
                cell_buffer.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(std::mem::take(&mut cell_buffer));
            }
            Event::FootnoteReference(label) => {
                print!("\x1b[2m[{}]\x1b[0m", label);
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                print!("\n\x1b[2m[{}]:\x1b[0m ", label);
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                println!();
            }
            Event::Html(html) => {
                if html.to_lowercase().contains("<img") {
                    print!("\x1b[2m[image]\x1b[0m");
                }
            }
            Event::InlineHtml(html) => {
                if html.to_lowercase().contains("<img") {
                    print!("\x1b[2m[image]\x1b[0m");
                }
            }
            Event::End(TagEnd::Paragraph) => {
                println!();
            }
            Event::SoftBreak => print!(" "),
            Event::HardBreak => println!(),
            Event::Rule => println!("\n\x1b[2m{}\x1b[0m", "─".repeat(42)),
            _ => {}
        }
    }
    println!();
}

fn main() {
    let cli = Cli::parse();
    let syntax_on = !cli.plain && cli.syntax_highlighting.to_lowercase() != "off";
    let markdown_on = !cli.plain && cli.markdown_preview.to_lowercase() != "off";
    let raw_mode = cli.show_all
        || cli.number_nonblank
        || cli.show_ends_v
        || cli.show_ends
        || cli.number
        || cli.squeeze_blank
        || cli.show_tabs
        || cli.show_nonprinting;

    let theme_key = resolve_theme(&cli.theme);

    let files: Vec<String> = if cli.files.is_empty() {
        vec!["-".to_string()]
    } else {
        cli.files.clone()
    };

    let mut exit_code = 0;

    for file in &files {
        match read_input(file) {
            Ok(mut content) => {
                if let Some(range) = &cli.range {
                    content = apply_range(&content, range);
                }
                if raw_mode {
                    print_raw(&content, &cli);
                } else {
                    let lower = file.to_lowercase();
                    let is_md = lower.ends_with(".md") || lower.ends_with(".markdown");
                    if is_md && markdown_on {
                        render_markdown(&content, theme_key);
                    } else if syntax_on {
                        highlight_and_print(&content, file, theme_key, &cli.language);
                    } else {
                        let _ = io::stdout().write_all(content.as_bytes());
                    }
                }
            }
            Err(e) => {
                eprintln!("cat: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }

    std::process::exit(exit_code);
}
