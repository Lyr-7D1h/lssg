# Lyr's Static Site Generator

The idea is to generate static html, css, js files based on custom markdown format.

This uses a custom markdown parser which will not necessarly follow official specifications. I'm making it to the point that it is usable for me, PR's are welcome!

## Usage

Install binary

```bash
git clone git@github.com:Lyr-7D1h/lssg.git
cd lssg
cargo install --path .
```

Generate static files

```bash
lssg {PATH_TO_INDEX_MARKDOWN_FILE} {PATH_TO_OUTPUT_FOLDER}
```

This is how you would generate lyrx from its content

```bash
cd examples/lyrx
lssg ./content/home.md ./build/ 
```

## LMarkdown (Lyr's Markdown)

## Architecture

```
Index markdown file path
    |
Sitetree: Parse index 
    |
Sitetree: Find resources (stylesheets, fonts, icons)
    |
Sitetree: Create nodes in tree by parsing resources
    |
Go through all nodes in tree
  if resources => copy
  if page => use modular HtmlRenderer to turn lmarkdown tokens into html, and write to file
```

## Roadmap
- Importing pages from notion support
- Code support
- Custom styling support
- Documentation module

## Known bugs
- references to root don't work 
