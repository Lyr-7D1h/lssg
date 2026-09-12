.PHONY: lyrx docs

lyrx:
	bacon run -- examples/lyrx/home.md ./build --no-media-optimization --preview --log-location

docs:
	bacon run -- docs/lssg.md ./build --no-media-optimization --preview --log-location

test:
	cargo test --all
