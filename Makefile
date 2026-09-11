.PHONY: lyrx docs

lyrx:
	cargo watch -x "run -- examples/lyrx/home.md ./build --no-media-optimization --preview --log-location"

docs:
	cargo run -- docs/lssg.md ./build --no-media-optimization --preview --log-location

test:
	cargo test --all
