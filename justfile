open-docs:
	./scripts/open-docs.sh

show-docs:
	just docdoc
	./scripts/open-docs.sh

docdoc:
	./scripts/docdoc.sh

gather:
	./scripts/gather.sh

gather-docs:
	just docdoc
	./scripts/gather-docs.sh

attach-probe:
	./scripts/attach-probe.sh

regenerate-text-pngs:
	./scripts/regenerate-text-pngs.sh

# Update led2d_graphics PNG expected output (host-only)
pngs-update-led2d-graphics:
	DEVICE_KIT_UPDATE_PNGS=1 cargo test --no-default-features --features host --test pngs led2d_graphics_png_matches_expected

# Update all expected PNG outputs (host-only)
pngs-update-all:
	DEVICE_KIT_UPDATE_PNGS=1 cargo test --no-default-features --features host --test pngs

# Full validation (docs + embedded + host PNGs)
verify-all:
	just docdoc
	cargo check-all
	just pngs-check-all

# Host-only PNG checks without updating
pngs-check-all:
	cargo test --no-default-features --features host --test pngs

# Generate video frames data (uses SANTA_VIDEO_PATH or SANTA_FRAMES_DIR)
video-frames:
	cargo xtask video-frames-gen > video_frames_data.rs

# Build an example for Pico 2 (ARM)
example name:
	cargo xtask example {{name}} --board pico2 --arch arm

# Build an example for Pico 2 (ARM) with WiFi
example-wifi name:
	cargo xtask example {{name}} --board pico2 --arch arm --wifi

# Build an example for Pico 1 with WiFi
example-pico1 name:
	cargo xtask example {{name}} --board pico1 --arch arm --wifi

# Build UF2 file for Pico 2 (ARM)
uf2 name:
	cargo xtask uf2 {{name}} --board pico2 --arch arm

# Build UF2 file for Pico 2 (ARM) with WiFi
uf2-wifi name:
	cargo xtask uf2 {{name}} --board pico2 --arch arm --wifi
