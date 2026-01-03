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

**Generated HTML:**
```html
<div class="default__centered">
  <p>This content will be centered.</p>
</div>
```

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

**Generated HTML:**
```html
<nav class="default__links">
  <a href="/project1.html" title="First project">
    <div class="default__links_box">Project 1</div>
  </a>
  <a href="/project2.html">
    <div class="default__links_box">Project 2</div>
  </a>
  <a href="/project3.html" title="Third project">
    <div class="default__links_box">Project 3</div>
  </a>
</nav>
```

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

**Generated HTML:**
```html
<div class="default__links_grid">
  <a href="/project1.html">
    <div class="default__links_grid_card">
      <div class="default__links_grid_card_cover">
        <img src="cover1.jpg" alt="Cover">
      </div>
      <h2 class="default__links_grid_card_title">Project 1</h2>
    </div>
  </a>
  <a href="/project2.html">
    <div class="default__links_grid_card">
      <div class="default__links_grid_card_cover">
        <img src="cover2.jpg" alt="Cover">
      </div>
      <h2 class="default__links_grid_card_title">Project 2</h2>
    </div>
  </a>
  <a href="/project3.html">
    <div class="default__links_grid_card">
      <h2 class="default__links_grid_card_title">Project 3</h2>
    </div>
  </a>
</div>
```

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

**Generated HTML:**
```html
<div class="default__carausel">
  <div class="default__carausel_main">
    <div class="default__carausel_item" onclick="default__toggleModal(event)">
      <img src="image1.jpg" alt="Image 1">
    </div>
  </div>
  <div class="default__carausel_other">
    <div class="default__carausel_item" onclick="default__toggleModal(event)">
      <img src="image2.jpg" alt="Image 2">
    </div>
    <div class="default__carausel_item" onclick="default__toggleModal(event)">
      <img src="image3.jpg" alt="Image 3">
    </div>
    <div class="default__carausel_item" onclick="default__toggleModal(event)">
      <img src="image4.jpg" alt="Image 4">
    </div>
  </div>
</div>
```

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

**Generated HTML:**
```html
<div class="default__sitetree">
  <div class="default__sitetree_folder">
    <a href="/docs/">docs/</a>
    <div class="default__sitetree_folder_content">
      <div class="default__sitetree_file">
        <a href="/docs/install.html">install</a>
      </div>
      <div class="default__sitetree_file">
        <a href="/docs/usage.html">usage</a>
      </div>
    </div>
  </div>
  <div class="default__sitetree_file">
    <a href="/about.html">about</a>
  </div>
</div>
```

**Attributes:**
- `ignore` - Comma-separated list of page names to exclude

**Notes:**
- Folders are sorted before files
- Within each group, items are sorted alphabetically
- Folders display with trailing `/`

## Custom Attributes

All custom elements support standard HTML attributes:

```markdown
<centered id="hero" class="my-class" style="color: red;">
Content here
</centered>
```

Any unrecognized HTML tag will be rendered as-is with its attributes:

```markdown
<myCustomTag data-value="123">
Custom content
</myCustomTag>
```
