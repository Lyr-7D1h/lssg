# Architecture

In short this is what happens when executing LSSG

```
Given index markdown file path
    |
Sitetree: Recursively find links to resources in parsed pages and stylesheets (stylesheets, fonts, icons, other pages)
    |
Sitetree: Add these resources as nodes into Sitetree
    |
Go through all nodes in tree
if resource 
    Copy resource
if page => use modular HtmlRenderer to turn lmarkdown tokens into html, and write to file
    HtmlRenderer: Create Domtree 
        |
    HtmlRenderer: Delecate modification of Domtree to modules based on LMarkdown Tokens
        |
    BlogModule: Render Token if applicable
        |
    DefaultModule: Fallback rendering of Token, it should render every kind of Token
```
