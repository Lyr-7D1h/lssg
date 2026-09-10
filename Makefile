.PHONY: lyrx docs

lyrx:
	cargo run -- examples/lyrx/home.md ./build --no-media-optimization --preview --log-location

docs:
	cargo run -- docs/lyrx.md ./build --no-media-optimization --preview --log-location

test:
	cargo test --all
