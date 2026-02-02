# Lyr's Static Site Generator

[![Crates.io](https://img.shields.io/crates/v/lssg.svg)](https://crates.io/crates/lssg)

A powerful recursive static site generator that builds entire websites from a single entry point.

LSSG intelligently traverses Markdown links to automatically discover and generate your complete site structure. Keep your content in Markdown as a single source of truth while LSSG handles the transformation into HTML, CSS, and JavaScript. Built with a custom Markdown parser, it offers seamless support for custom HTML elements and provides fine-grained control over your static site generation workflow.

Some features included:
- **Recursive Link Discovery**: Automatically follows and generates pages from Markdown links
- **Custom HTML Elements**: Built-in components like `<links>`, `<centered>`, `<carousel>`, and `<gallery>` for rich layouts
- **Automatic Resource Management**: Discovers and copies CSS, JavaScript, images, fonts, and other assets
- **Flexible Navigation**: Support for breadcrumbs and side menus with customizable display names
- **Blog Module**: RSS feed generation, post management, and date-based sorting
- **TOML Configuration**: Fine-grained control via inline comments in markdown files
- **Remote Content Support**: Generate sites from remote markdown files (GitHub, etc.)
- **Custom Markdown Parser**: Enhanced markdown with support for tables, section links, and HTML integration
- **SEO-Friendly**: Meta tag customization, language settings, and semantic HTML structure
- **Modular Architecture**: Extensible system for custom rendering and processing modules


## Documentation

- [Installation](https://lssg.lyrx.dev/install) - How to install LSSG
- [Usage](https://lssg.lyrx.dev/usage) - How to use LSSG to generate static sites
- [Configuration](https://lssg.lyrx.dev/configuration) 
- [Custom Html Element](https://lssg.lyrx.dev/custom_elements) 
- [LMarkdown](https://lssg.lyrx.dev/lmarkdown) - Learn about the custom markdown format
- [Architecture](https://lssg.lyrx.dev/architecture) - Understand how LSSG works internally
- [Roadmap](https://lssg.lyrx.dev/roadmap) - Future plans and completed features
- [Tutorials](https://lssg.lyrx.dev/tutorials) - Tutorials relevant to Static Site Generation
