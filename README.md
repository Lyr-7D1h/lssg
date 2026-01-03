<!--
title = "LSSG"
head=[
  '<script defer src="https://analytics.lyrx.dev/script.js" data-website-id="a3641f2d-876d-4971-97d3-2cb6c57a762b"></script>'
]

[nav]
kind = "sidemenu"
include_root = true
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
# Lyr's Static Site Generator

Recursively generate static html, css, js files from a single markdown file.

This uses a custom Markdown parser which will not necessarily follow [official specifications](https://commonmark.org/). I'm making it to support custom html elements inside of the markdown better. Writing a custom parser makes bugs more likely, PR's are welcome!

## Documentation

- [Installation](docs/install.md) - How to install LSSG
- [Usage](docs/usage.md) - How to use LSSG to generate static sites
- [Configuration](docs/configuration.md) 
- [Custom Html Element](docs/custom_elements.md) 
- [LMarkdown](docs/lmarkdown.md) - Learn about the custom markdown format
- [Architecture](docs/architecture.md) - Understand how LSSG works internally
- [Roadmap](docs/roadmap.md) - Future plans and completed features
- [Tutorials](docs/tutorials.md) - Tutorials relevant to Static Site Generation
