# Roadmap

## Planned Features

- YAML frontmatter metadata support
- Trees like the ones https://owickstrom.github.io/the-monospace-web/
- Tables
- [Section links](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#section-links)
- [Emoji support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#using-emoji)
- [Footnote support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#footnotes)
- [Alert support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)
- Code highlighting support
- Use [Shadow dom](https://developer.mozilla.org/en-US/docs/Web/API/Web_components/Using_shadow_DOM) for encapsulating imported html pages to prevent conflicting js and css
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
- Improve blog module
    - RSS functionality
    - tags
    - Better index page
        - Show and sort by date
        - Small blog post summary
- Statistics module
    - Fetch github statistics
- Multi platform support
    - Make releases for other platforms

## Completed

- ~~Custom styling support~~
- ~~Make default options root of Attributes (don't require [default] block)~~
- ~~Html macro~~
