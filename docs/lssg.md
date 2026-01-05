<!--
title = "LSSG"
head=[
  '<script defer src="https://analytics.lyrx.dev/script.js" data-website-id="a3641f2d-876d-4971-97d3-2cb6c57a762b"></script>'
]
root=true

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


Recursively generate static html, css, js files from a single markdown file.

This generator has many features and makes it easy to keep a single source of truth (your markdown files) and generate a whole web of static files based on links within your entry markdown file.

This uses a custom Markdown parser which will not necessarily follow [official specifications](https://commonmark.org/). I'm making it to support html elements inside the markdown better as well as support other syntaxes and to support high customization of the formatting language used. Writing a custom parser makes bugs more likely, PR's are welcome!
