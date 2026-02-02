# Lyr's Static Site Generator

<p style="text-align: left">
[<i class="fa-brands fa-dochub"></i>Docs](https://lssg.lyrx.dev)
[<i class="fa-brands fa-github"></i>Repository](https://github.com/Lyr-7D1h/lssg)
[<i class="fa-brands fa-rust"></i>Crates.io](https://crates.io/crates/lssg)
</p>


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
