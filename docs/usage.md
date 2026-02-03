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

To automatically watch for changes and preview your site, use the preview mode:

```bash
lssg ./examples/lyrx/home.md ./build --preview
```

This will:
- Generate your static site
- Start a local preview server at http://localhost:8000
- Watch for file changes and regenerate automatically
- Serve your site with proper MIME types and 404 page support

You can also specify a custom port:

```bash
lssg ./examples/lyrx/home.md ./build --preview --port 3000
```

**Watch-only mode**

If you prefer to use your own server, you can watch for changes without the preview server:

```bash
lssg ./examples/lyrx/home.md ./build --watch
```

This will watch the parent folder `./examples/lyrx/` for file changes and regenerate the site automatically.

**Custom watch path**

You can specify a custom directory to watch:

```bash
lssg ./examples/lyrx/home.md ./build --preview --watch-path ./content
```

This is useful when your content is spread across different directories or you want to watch a specific subset of files.

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
  -v, --version                  Print version information
  -s, --single-page              Print output of a single page
  -a, --ast                      Print ast tokens of a single page
  -l, --log <LOG>                "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
  -n, --no-media-optimization    Enable media optimization (images and videos)
  -w, --watch                    Watch for file changes and regenerate automatically
      --watch-path <WATCH_PATH>  Custom path to watch for file changes (defaults to input file's parent directory)
  -p, --preview                  Start a preview server to view the generated site (Note: implicitely also runs --watch)
      --port <PORT>              Port for the preview server (default: 8000) [default: 8000]
  -h, --help                     Print help (see more with '--help')
```