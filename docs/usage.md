# Usage

## Generate static files

```bash
lssg {PATH_TO_INDEX_MARKDOWN_FILE} {PATH_TO_OUTPUT_FOLDER}
```

This is how you would generate lyrx from its content

```bash
lssg ./examples/lyrx/home.md ./build
```

**Live reload and preview**

To automatically watch for changes use

```bash
lssg ./examples/lyrx/home.md ./build --watch
```

This will watch the parent folder `./examples/lyrx/` for file changes.

You can use a simple html live reload server to preview changes made as you write like [live-server](https://github.com/tapio/live-server) 

```bash
live-server ./build`
```

Now you can view the changes you make at http://localhost:8080

## Using remote markdown files

You can also use links to markdown to generate content

```bash
lssg https://raw.githubusercontent.com/Lyr-7D1h/lssg/master/examples/lyrx/home.md ./build
```

> [!NOTE]
> Any local links from the input markdown file to other markdown files have to be contained within the parent folder of your input markdown file

## More options

See `lssg --help` for more options

```
Lyr's Static Site Generator - Command Line Interface

Usage: lssg [OPTIONS] <INPUT> [OUTPUT]

Arguments:
  <INPUT>   a reference to the first markdown input file this can either be a path (eg. ./my_blog/index.md) or an url (eg. http://github.com/project/readme.md)
  [OUTPUT]  path to put the static files into, any needed parent folders are automatically created

Options:
  -v, --version                Print version information
  -s, --single-page            Print output of a single page
  -a, --ast                    Print ast tokens of a single page
  -l, --log <LOG>              "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
  -n, --no-media-optimization  Enable media optimization (images and videos)
  -w, --watch                  Watch for file changes and regenerate automatically
  -h, --help                   Print help (see more with '--help')
```