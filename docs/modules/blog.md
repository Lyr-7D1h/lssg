# Blog Module

The Blog module adds blogging capabilities to your static site, including post indexing, RSS feed generation, and date management. It automatically detects blog root pages and their posts.

## Configuration Sections

The Blog module uses two different configuration sections depending on whether a page is a blog root (index) or a blog post.

### Blog Root Options

Use `[blog.root]` for the main blog index page:

#### `rss.enabled`
- **Type:** Boolean
- **Default:** `false`
- **Description:** Enable RSS feed generation for this blog.

#### `rss.path`
- **Type:** String
- **Default:** `"feed.xml"`
- **Description:** Output filename for the RSS feed.

#### `rss.title`
- **Type:** String
- **Default:** `"Feed"`
- **Description:** Title for the RSS feed.

#### `rss.description`
- **Type:** String
- **Default:** `"My feed"`
- **Description:** Description for the RSS feed.

#### `rss.host`
- **Type:** String
- **Optional**
- **Description:** Base URL for the blog (e.g., `"https://example.com"`). Required for RSS feed generation to create absolute URLs.

#### `rss.language`
- **Type:** String
- **Optional**
- **Description:** Language code for the RSS feed (e.g., `"en"`, `"de"`). Helps feed readers display content appropriately.

#### `rss.last_build_date_enabled`
- **Type:** Boolean
- **Default:** `true`
- **Description:** When enabled, sets the RSS feed's last build date to the most recent post's creation date.

#### `use_fs_dates`
- **Type:** Boolean
- **Default:** `false`
- **Description:** Use filesystem dates (file modification/creation times) instead of manual date configuration.

**Example Blog Root:**
```markdown
<!--
title = "My Blog"

[blog.root]
use_fs_dates = true

[blog.root.rss]
enabled = true
title = "My Blog RSS Feed"
description = "Latest posts from my blog"
host = "https://example.com"
language = "en"
path = "feed.xml"
-->

# My Blog

Welcome to my blog...
```

### Blog Post Options

Use `[blog.post]` for individual blog posts:

#### `render`
- **Type:** Boolean
- **Default:** `true`
- **Description:** Enable blog-specific rendering for this post. If false, the page will still be indexed but rendered as a normal page.

#### `created_on`
- **Type:** String (ISO date or YYYY-MM-DD)
- **Optional**
- **Description:** When the article was first published.

**Example:**
```toml
created_on = "2025-01-15"
# or
created_on = "2025-01-15T10:30:00Z"
```

#### `modified_on`
- **Type:** String (ISO date or YYYY-MM-DD)
- **Optional**
- **Description:** When the article was last modified.

#### `tags`
- **Type:** Array of Strings
- **Optional**
- **Description:** Tags or categories for the blog post.

#### `summary`
- **Type:** String
- **Optional**
- **Description:** Short summary of the post for use in listings and RSS feeds.

**Example Blog Post:**
```markdown
<!--
title = "How to Build a Static Site"

[blog.post]
render = true
created_on = "2025-12-29"
modified_on = "2025-12-29"
tags = ["tutorial", "web development", "static sites"]
summary = "A comprehensive guide to building static websites"

[meta]
description = "Learn how to build static sites from scratch"
-->

# How to Build a Static Site

Your blog post content here...
```

## Features

### Automatic Date Display

When a blog post has date information, the Blog module automatically inserts a formatted date display below the first H1 heading.

### Meta Tags

The module automatically adds article meta tags for better SEO:
- `article:published_time` - From `created_on`
- `article:modified_time` - From `modified_on`

### RSS Feed

When RSS is enabled, the module generates a standards-compliant RSS feed containing:
- All published posts under the blog root
- Post titles, links, and publication dates
- Post summaries (if provided)
- Automatic GUID generation for each post

### Blog Styling

The module automatically includes `blog.css` stylesheet for all blog posts, providing consistent styling for blog-specific elements like date displays.

## Example Blog Structure

```
blog/
├── index.md          # Blog root with [blog] config
├── post1.md          # Blog post
├── post2.md          # Blog post
└── drafts/
    └── draft1.md     # Not indexed (not linked from root)
```

**blog/index.md:**
```markdown
<!--
title = "My Blog"

[blog]
[blog.rss]
enabled = true
title = "My Blog"
-->

# My Blog

[First Post](./post1.md)
[Second Post](./post2.md)
```

**blog/post1.md:**
```markdown
<!--
title = "My First Post"

[blog]
created_on = "2025-01-01"
tags = ["introduction"]
-->

# My First Post

Content here...
```

## Notes

- Posts are automatically discovered by following links from the blog root page
- Dates can be in ISO format or simple YYYY-MM-DD format
- The RSS feed is automatically linked in the blog root page
- Use `use_fs_dates = true` if you want to automatically use file timestamps instead of manual dates
