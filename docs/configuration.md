# Configuration

LSSG uses a modular architecture where different modules handle specific rendering and processing tasks. Configure them via TOML blocks at the start of markdown files.

## Available Modules

### [Default Module](modules/default.md)

Core rendering module for standard markdown and HTML structure. Required for all pages.

- Standard markdown elements (headings, paragraphs, links, images, code)
- Page structure (head, body, footer)
- Meta tags and SEO
- Code syntax highlighting
- Customizable titles, language, footer

[→ Full Documentation](modules/default.md)

### [Blog Module](modules/blog.md)

Adds blogging capabilities with post management and RSS feeds.

- Post discovery and indexing
- RSS feed generation
- Date management and display
- Tags and summaries
- Article meta tags

[→ Full Documentation](modules/blog.md)

### [Media Module](modules/media.md)

Automatic optimization for images and videos.

- Image compression and resizing
- WebP conversion
- Video optimization (FFmpeg)
- Configurable quality settings

[→ Full Documentation](modules/media.md)

### [External Module](modules/external.md)

Import complete HTML sites from remote ZIP archives.

- Downloads and extracts ZIP files
- Integrates HTML, CSS, and resources
- Preserves folder structure
- Useful for embedding external documentation

[→ Full Documentation](modules/external.md)

## Configuration Example

```markdown
<!--
title = "My Page"
language = "en"

[blog]
created_on = "2025-12-29"
tags = ["tutorial"]

[media]
optimize_images = true
image_quality = 85
-->

# Page Content
```

## Execution Order

1. **External** - Downloads external content
2. **Blog** - Adds blog features
3. **Media** - Optimizes media
4. **Default** - Renders remaining content (fallback)