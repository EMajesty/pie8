.PHONY: all check clean sd

all:
	./scripts/build.sh

check:
	./scripts/check.sh

clean:
	cargo clean
	rm -rf build

# Usage: make sd SD_BOOT=/path/to/empty/mounted/fat-partition
sd: all
	./scripts/prepare-sd.sh "$(SD_BOOT)"
