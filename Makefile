SAMPLE ?= samples/fullmoon-fulllife.mp3

.PHONY: map serve build clean

# Analyse the audio and write map.json + audio.mp3 into the frontend.
map:
	cargo run -p backend --release -- $(SAMPLE)

# Generate the map, then serve the frontend at http://localhost:8080
serve: map
	cd crates/frontend && trunk serve

build: map
	cd crates/frontend && trunk build --release

clean:
	cargo clean
	rm -rf crates/frontend/dist
