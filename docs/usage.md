# Usage

## Generate static files

```bash
lssg {PATH_TO_INDEX_MARKDOWN_FILE} {PATH_TO_OUTPUT_FOLDER}
```

This is how you would generate lyrx from its content

```bash
lssg ./examples/lyrx/home.md ./build
```

You can use a simple html live reload server to view changes as you develop like [live-server](https://github.com/tapio/live-server)

## Using remote markdown files

You can also use links to markdown to generate content

```bash
lssg https://raw.githubusercontent.com/Lyr-7D1h/lssg/master/examples/lyrx/home.md ./build
```

> [!NOTE]
> Any local links from the input markdown file to other markdown files have to be contained within the parent folder of your input markdown file
