# Architecture

In short this is what happens when executing LSSG

```
Given index markdown file path
    |
Sitetree: Recursively find links to resources in parsed pages and stylesheets (stylesheets, fonts, icons, other pages)
    |
Sitetree: Add these resources as nodes into Sitetree
    |
RenderModules: Run init(), modifying site_tree
    |
RenderModules: Run after_init(), viewing the final site_tree
    |
Go through all nodes in tree
if resource 
    Copy resource to output
if page 
    HtmlRenderer: Create Domtree 
        |
    RenderModules: Run render_page(), rendering a whole page and modifying the domtree
        |
    RenderModules: Run render_token(), rendering a single lmarkdown token to html 
        |
    RenderModules: Run after_render(), modifying the page based on what was rendered
        |
    HtmlRenderer: Clean domtree
        |
    Write to output
```
