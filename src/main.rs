use clap::Parser;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser as MdParser, Tag, TagEnd};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

#[derive(Parser)]
#[command(name = "rcat", version, about = "cat, but with syntax highlighting and markdown preview")]
struct Cli {
    #[arg(short = 'A')]
    show_all: bool,
    #[arg(short = 'b')]
    number_nonblank: bool,
    #[arg(short = 'e')]
    show_ends_v: bool,
    #[arg(short = 'E')]
    show_ends: bool,
    #[arg(short = 'n')]
    number: bool,
    #[arg(short = 's')]
    squeeze_blank: bool,
    #[arg(short = 'T')]
    show_tabs: bool,
    #[arg(short = 'v')]
    show_nonprinting: bool,
    #[arg(long = "syntax-highlighting", default_value = "on")]
    syntax_highlighting: String,
    #[arg(long = "markdown-preview", default_value = "on")]
    markdown_preview: String,
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
                        out.push((low as u8) as char);
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

fn find_syntax<'a>(ss: &'a SyntaxSet, filename: &str, content: &str) -> &'a SyntaxReference {
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

fn highlight_and_print(content: &str, filename: &str) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = find_syntax(&ss, filename, content);
    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);
    let mut out = String::with_capacity(content.len() * 2);

    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, &ss).unwrap_or_default();
        out.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
    }
    out.push_str("\x1b[0m");
    let _ = io::stdout().write_all(out.as_bytes());
}

fn render_markdown(content: &str) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];

    let parser = MdParser::new(content);
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buffer = String::new();

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
            Event::Start(Tag::Strong) => print!("\x1b[1m"),
            Event::End(TagEnd::Strong) => print!("\x1b[22m"),
            Event::Start(Tag::Emphasis) => print!("\x1b[3m"),
            Event::End(TagEnd::Emphasis) => print!("\x1b[23m"),
            Event::Start(Tag::Strikethrough) => print!("\x1b[9m"),
            Event::End(TagEnd::Strikethrough) => print!("\x1b[29m"),
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
                print!("\x1b[38;5;222m{}\x1b[0m", text);
            }
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
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
            Event::Start(Tag::BlockQuote) => {
                print!("\x1b[2;3m▏ ");
            }
            Event::End(TagEnd::BlockQuote) => {
                println!("\x1b[0m");
            }
            Event::Start(Tag::Link { .. }) => {
                print!("\x1b[4;34m");
            }
            Event::End(TagEnd::Link) => {
                print!("\x1b[0m");
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
    let syntax_on = cli.syntax_highlighting.to_lowercase() != "off";
    let markdown_on = cli.markdown_preview.to_lowercase() != "off";
    let raw_mode = cli.show_all
        || cli.number_nonblank
        || cli.show_ends_v
        || cli.show_ends
        || cli.number
        || cli.squeeze_blank
        || cli.show_tabs
        || cli.show_nonprinting;

    let files: Vec<String> = if cli.files.is_empty() {
        vec!["-".to_string()]
    } else {
        cli.files.clone()
    };

    let mut exit_code = 0;

    for file in &files {
        match read_input(file) {
            Ok(content) => {
                if raw_mode {
                    print_raw(&content, &cli);
                } else {
                    let lower = file.to_lowercase();
                    let is_md = lower.ends_with(".md") || lower.ends_with(".markdown");
                    if is_md && markdown_on {
                        render_markdown(&content);
                    } else if syntax_on {
                        highlight_and_print(&content, file);
                    } else {
                        let _ = io::stdout().write_all(content.as_bytes());
                    }
                }
            }
            Err(e) => {
                eprintln!("rcat: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }

    std::process::exit(exit_code);
}
