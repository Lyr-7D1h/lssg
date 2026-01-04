# Custom HTML Elements

LSSG provides custom HTML elements that you can use directly in your markdown files to create rich layouts and components.

## `<centered>`

Center content horizontally.

**Markdown:**
```markdown
<centered>
This content will be centered.
</centered>
```

**Result:**

<centered>
This content will be centered.
</centered>

## `<links>`

Create styled link collections in different layouts.

### Links Boxes

Display links as styled boxes in a navigation layout.

**Markdown:**
```markdown
<links boxes>
[Project 1](./project1.md "First project")
[Project 2](./project2.md)
[Project 3](./project3.md "Third project")
</links>
```

**Result:**

<links boxes>
[Installation](./install.md "How to install LSSG")
[Usage](./usage.md "How to use LSSG")
[Configuration](./configuration.md "Configure your site")
</links>

### Links Grid

Display links as a grid of cards with optional cover images.

**Markdown:**
```markdown
<links grid>
[![Cover](cover1.jpg) Project 1](./project1.md)
[![Cover](cover2.jpg) Project 2](./project2.md)
[Project 3](./project3.md)
</links>
```

**Result:**

<links grid>
[LMarkdown](./lmarkdown.md)
[Architecture](./architecture.md)
[Tutorials](./tutorials.md)
</links>

**Notes:**
- If link content starts with an image, it becomes the cover
- Cover images are automatically scaled (width: 100%, height: auto)
- SVG covers get viewBox attributes for proper scaling

## `<carousel>`

Create an image carousel with a main display and thumbnails.

**Markdown:**
```markdown
<carousel>
![Image 1](image1.jpg)
![Image 2](image2.jpg)
![Image 3](image3.jpg)
![Image 4](image4.jpg)
</carousel>
```

**Result:**

<carousel>
![Bear Image](./custom_elements/bear.jpg)
![Bear Image](./custom_elements/bear.jpg)
![Bear Image](./custom_elements/bear.jpg)
![Bear Image](./custom_elements/bear.jpg)
</carousel>

**Notes:**
- First image appears in main display
- Remaining images appear as thumbnails
- Items are clickable with modal functionality

## `<sitetree>`

Generate a hierarchical site navigation tree.

**Markdown:**
```markdown
<sitetree>
```

With ignore list:
```markdown
<sitetree ignore="404,blog">
```

**Result:**

<sitetree></sitetree>

**Attributes:**
- `ignore` - Comma-separated list of page names to exclude

**Notes:**
- Folders are sorted before files
- Within each group, items are sorted alphabetically
- Folders display with trailing `/`