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

### `<links boxes>`

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

### `<links grid>`

Display links as a grid of cards with optional cover images.

**Markdown:**
```markdown
<links grid>
[![Bear Image](./custom_elements/bear.jpg) LMarkdown](./lmarkdown.md)
[![Bear Image](./custom_elements/bear.jpg) Architecture](./architecture.md)
[Tutorials](./tutorials.md)
</links>
```

**Result:**

<links grid>
[![Bear Image](./custom_elements/bear.jpg) LMarkdown](./lmarkdown.md)
[![Bear Image](./custom_elements/bear.jpg) Architecture](./architecture.md)
[Tutorials](./tutorials.md)
</links>

**Notes:**
- If link content starts with an image, it becomes the cover
- Cover images are automatically scaled (width: 100%, height: auto)
- SVG covers get viewBox attributes for proper scaling

## `<carousel>`

Create an image carousel with a main display and thumbnails. Supports images and 3D model-viewer elements.

**Markdown:**
```markdown
<carousel>
<model-viewer alt="JBL Holder Band" src="./models/band.gltf" ar shadow-intensity="1" camera-controls touch-action="pan-y"></model-viewer>
<model-viewer alt="JBL Holder Frame" src="./models/jbl_holder.gltf" ar shadow-intensity="1" camera-controls touch-action="pan-y"></model-viewer>
![Image 1](image1.jpg)
![Image 2](image2.jpg)
</carousel>
```

**Result:**

<carousel>
<model-viewer alt="JBL Holder Band" src="./custom_elements/jbl_holder.gltf" ar shadow-intensity="1" camera-controls touch-action="pan-y"></model-viewer>
![Bear Image](./custom_elements/bear.jpg)
![Bear Image](./custom_elements/bear.jpg)
</carousel>

## `<model-viewer>`

Display interactive 3D models using [Google's model-viewer component](https://modelviewer.dev/). Supports GLTF/GLB formats with camera controls, AR capabilities, and auto-rotation. 

Any resource links referenced in `model-viewer` attributes are preserved in the rendered output.

**Markdown:**
```markdown
<centered>
  <model-viewer 
    style="height:400px"
    alt="JBL Speaker Holder" 
    src="./models/jbl_holder.gltf" 
    ar 
    shadow-intensity="1" 
    camera-controls 
    touch-action="pan-y">
  </model-viewer>
</centered>
```

**Result:**

<centered>
<model-viewer style="height: 400px" alt="Bike Speaker Holder" src="./custom_elements/jbl_holder.gltf" ar shadow-intensity="1" camera-controls touch-action="pan-y"></model-viewer>
</centered>

**Common Attributes:**
- `src` - Path to the 3D model file (.gltf or .glb)
- `alt` - Alternative text description
- `camera-controls` - Enable mouse/touch camera controls
- `ar` - Enable AR viewing on supported devices
- `shadow-intensity` - Shadow darkness (0-1)
- `touch-action` - CSS touch action policy
- `auto-rotate` - Automatically rotate the model
- `auto-rotate-delay` - Delay before starting rotation (ms)


## `<sitetree>`

Generate a hierarchical site navigation tree.

**Markdown:**
```markdown
<sitetree>
```

With ignore list:
```markdown
<sitetree ignore="404">
```

**Result:**

<sitetree></sitetree>

**Attributes:**
- `ignore` - Comma-separated list of page names to exclude

**Notes:**
- Folders are sorted before files
- Within each group, items are sorted alphabetically
- Folders display with trailing `/`