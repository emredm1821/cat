# cat
Linux "cat" command-line tool made in Windows with Syntax Highlighting and Markdown Preview in Rust.

## Preview
<img width="1359" height="719" alt="Example" src="https://github.com/user-attachments/assets/b170df4c-cafb-4fe1-b48d-0ffae336f21b" />

## Features

- **Syntax highlighting**: for a wide range of programming languages, powered by [`syntect`](https://github.com/trishume/syntect)
- **Markdown preview**: headings, bold/italic/strikethrough, lists, code blocks, blockquotes, and links rendered directly in the terminal
- **Drop-in `cat` replacement**: supports classic flags like `-A`, `-b`, `-e`, `-E`, `-n`, `-s`, `-T`, `-v`
- **Fast**: written in Rust, compiled with LTO and full optimizations
- **Toggleable**: disable syntax highlighting or markdown rendering with a single flag when you just want raw output
- **Stdin support**: pipe input directly, just like the real `cat`
- **No crashes on binary/non-UTF-8 files**: falls back gracefully instead of erroring out

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

For CMD:
```bat
curl -sL https://raw.githubusercontent.com/emredm1821/cat/main/install/cmd.bat -o "%TEMP%\install.bat" && call "%TEMP%\install.bat"
```

For Powershell:
```ps1
irm https://raw.githubusercontent.com/emredm1821/cat/main/install/powershell.ps1 | iex
```

## Building from source

Requires [Rust](https://www.rust-lang.org/tools/install) and Cargo.

```sh
git clone https://github.com/emredm1821/cat.git
cd cat
cargo build --release
```

The compiled binary will be located at `target/release/cat.exe`.

## License

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

This project is licensed under the GNU General Public License v3.0 — see the [LICENSE](LICENSE) file for details.
