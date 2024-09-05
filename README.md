# Lyr's Static Site Generator

Recursively generate static html, css, js files based on a single markdown file.

This uses a custom markdown parser which will not necessarily follow official specifications. I'm making it to the point that it is usable for me, PR's are welcome! See `LMarkdown` down for more information.

## Install

**Install from cargo**

```
cargo install lssg
```

**Install from git**

```bash
git clone git@github.com:Lyr-7D1h/lssg.git
cd lssg
cargo install --path .
```

## Usage

Generate static files

```bash
lssg {PATH_TO_INDEX_MARKDOWN_FILE} {PATH_TO_OUTPUT_FOLDER}
```

This is how you would generate lyrx from its content

```bash
cd examples/lyrx
lssg ./content/home.md ./build
```

You can use a simple html live reload server to view changes as you develop like [live-server](https://github.com/tapio/live-server)

You can also use links to markdown to generate content

```bash
lssg https://raw.githubusercontent.com/Lyr-7D1h/lssg/master/examples/lyrx/home.md ./build
```

> [!NOTE]
> Any links from the input markdown file to other markdown files have to be contained within the parent folder of your input markdown file

## LMarkdown (Lyr's Markdown)

LMarkdown tries to follow [Commonmark](https://commonmark.org/) markdown specifications although deviating wherever it makes sense to make page rendering easier.

Structure of a lmarkdown file:

```markdown
<!--
{MODULE_CONFIG}
-->
{MARKDOWN}
```

eg.

```markdown
<!--
[default]
title="This is the html title"
[blog]
root = true
-->
<!--
    The first comment on a page is seen as module configuration and is parsed as toml 
    it has the following format:

    [{module_identifier}]
    {options}
-->

# Just some header in file

<!-- All HTML comments are ignore in output except starting comments like seen above -->

<!-- The following will generate `http://{root}/test` url based on the markdown file -->

[Check out my other page](./test.md)

<!-- So this in html will turn into `<a href="./test">Check out my other page</a>` -->
```

## Architecture

In short this is what happens when executing LSSG

```
Given index markdown file path
    |
Sitetree: Recursively find links to resources in parsed pages and stylesheets (stylesheets, fonts, icons, other pages)
    |
Sitetree: Add these resources as nodes into Sitetree
    |
Go through all nodes in tree
if resources 
    Copy resource
if page => use modular HtmlRenderer to turn lmarkdown tokens into html, and write to file
    HtmlRenderer: Create Domtree 
        |
    HtmlRenderer: Delecate modification of Domtree to modules based on LMarkdown Tokens
        |
    BlogModule: Render Token if applicable
        |
    DefaultModule: Fallback rendering of Token, it should render every kind of Token
```

## Roadmap
- Trees like the ones https://owickstrom.github.io/the-monospace-web/
- [Section links](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#section-links)
- [Emoji support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#using-emoji)
- [Footnote support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#footnotes)
- [Alert support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)
- Code highlighting support
- Support for relative base website.com/blog/index.html
    - See `<base href="http://yourdomain.com/">`
- Update documentation
- Make async
    - reqwest calls async
- Add recovery and logging instead of panicking
    - panic on broken link
- Download and install links to external resources (fonts, CSS, enc.)
- Make importing pages from notion easier
- Don't load all files into memory, might cause issues for large resource files or big sites
- Add file minification for CSS
- Documentation module
- Multi platform support
    - Make releases for other platforms
~~- Custom styling support~~
~~- Make default options root of Attributes (don't require [default] block)~~
~~- Html macro
