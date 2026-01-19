# Roadmap

## Planned Features

- Full Github Markdown Flavor support https://github.github.com/gfm/#raw-html
- YAML frontmatter metadata support
- Trees like the ones https://owickstrom.github.io/the-monospace-web/
- [Emoji support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#using-emoji)
- [Footnote support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#footnotes)
- [Alert support](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)
- Code highlighting support
- Use [Shadow dom](https://developer.mozilla.org/en-US/docs/Web/API/Web_components/Using_shadow_DOM) for encapsulating imported html pages to prevent conflicting js and css
- Transforms html into lmarkdown
    - enables archiving and a consistent view of content for non markdown pages. Think of old blogs or other content that you can integrate in your current setup
- Support for relative base website.com/blog/index.html
    - See `<base href="http://yourdomain.com/">`
- Update documentation
- Make async
    - reqwest calls async
- Download and install links to external resources (fonts, CSS, enc.)
- Don't load all files into memory, might cause issues for large resource files or big sites
- Add file minification for CSS and JS
- Improve blog module
    - tags
    - Index page
        - Show and sort by date
        - Small blog post summary
- Statistics module
    - Fetch github statistics
- Multi platform support
    - Make releases for other platforms

## Completed

- ~~RSS functionality~~
- ~~Tables~~
- ~~[Section links](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#section-links)~~
- ~~Custom styling support~~
- ~~Make default options root of Attributes (don't require [default] block)~~
- ~~Html macro~~
