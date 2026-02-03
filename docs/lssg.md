<!--
title = "LSSG"
head=[
  '<script defer src="https://analytics.lyrx.dev/script.js" data-website-id="67a7b102-54ff-42a8-9ab3-15ffdf2fb688"></script>'
]
root=true


[[nav]]
kind = "sidemenu"
include_root = true
ignore=["404"]
[nav.name_map]
README = "Home"
install = "Install"
usage = "Usage"
configuration = "Configuration"
custom_elements = "Custom Elements"
lmarkdown = "Lyr's Markdown"
architecture = "Architecture"
roadmap = "Roadmap"
tutorials = "Tutorials"
how_to_host_static_files = "How to host static files"

[meta]
description="Lyr's Static Site Generator documentation"
author="Lyr, lyr-7d1h@pm.me"
keywords="blog,ssg,static-site-generation,technology,projects,software"
subject="technology"
image="https://lyrx.dev/icon.png"
-->

[](./404.md)
[](./lib/fontawesome.css)
[](./lib/fa-solid.css)
[](./lib/fa-brands.css)

[](./install.md)
[](./usage.md)
[](./configuration.md) 
[](./custom_elements.md) 
[](./lmarkdown.md)
[](./architecture.md)
[](./roadmap.md)
[](./tutorials.md)

# Lyr's Static Site Generator

<p style="text-align: left">
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
