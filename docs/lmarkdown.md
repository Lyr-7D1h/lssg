# LMarkdown (Lyr's Markdown)

LMarkdown tries to follow [Commonmark](https://commonmark.org/) markdown specifications although deviating wherever it makes sense to make page rendering easier. Support for [Github Flavor Markdown](https://github.github.com/gfm/) is **wip**

## Structure

Structure of a lmarkdown file:

```markdown
<!--
{MODULE_CONFIG}
-->
{MARKDOWN}
```

For how to define values in `MODULE_CONFIG` see [Modules](./configuration.md)

## Differences with CommonMark

- Partial support for [github style tables](https://github.github.com/gfm/#tables-extension-)
- No newlines needed for inserting html
- [TOML](https://toml.io/en/) configuration comment on top of a markdown document

## Example

```markdown
<!--
[default]
title="This is the html title"
[post]
root = true
-->
<!--
    The first comment on a page is seen as module configuration and is parsed as toml 
    it has the following format:

    [{module_identifier}]
    {options}
-->

# Just some header in file

<!-- All HTML comments are ignore in output except starting comments like seen above -->

<!-- The following will generate `http://{root}/test` url based on the markdown file -->

[Check out my other page](./test.md)

<!-- So this in html will turn into `<a href="./test">Check out my other page</a>` -->
```
