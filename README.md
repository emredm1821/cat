# cat
Linux "cat" command-line tool made in Windows with Syntax Highlighting and Markdown Preview in Rust.

## Preview
<img width="1359" height="719" alt="Example" src="https://github.com/user-attachments/assets/b170df4c-cafb-4fe1-b48d-0ffae336f21b" />

## Usage

```
cat [options] [file...]
```

| Option | Description |
|---|---|
| `-A` | Show all characters (equivalent to `-v`, `-E`, `-T` combined) |
| `-b` | Number non-blank output lines |
| `-e` | Display `$` at end of each line and show non-printing characters (equivalent to `-v`, `-E`) |
| `-E` | Display `$` at the end of each line |
| `-n` | Number all output lines, including blank lines |
| `-s` | Squeeze multiple adjacent blank lines into a single blank line |
| `-T` | Display TAB characters as `^I` |
| `-v` | Show non-printing characters (except for tabs and end-of-line) |
| `--syntax-highlighting=<on\|off>` | Toggle syntax highlighting (default: `on`) |
| `--markdown-preview=<on\|off>` | Render `.md`/`.markdown` files as formatted output in the terminal (default: `on`) |
| `-h`, `--help` | Print help information |
| `-V`, `--version` | Print version information |

### Examples

```sh
cat file.rs
cat -n file.txt
cat --markdown-preview=off README.md
cat --syntax-highlighting=off file.py
cat file.txt | cat -A
```

## Installation

 
