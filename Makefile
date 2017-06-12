TARGET?=debug

.PHONY: all force clean

all: target/$(TARGET)/nero $(patsubst plugins/%,target/$(TARGET)/libnero_%.so,$(wildcard plugins/*))

force:
	@make -B

clean:
	rm -f target/$(TARGET)/nero
	rm -f $(patsubst plugins/%,target/$(TARGET)/libnero_%.so,$(wildcard plugins/*))
	rm -f $(patsubst plugins/%,target/$(TARGET)/libnero_%.d,$(wildcard plugins/*))

target/debug/nero:
	@cargo build

target/release/nero:
	@cargo build --release

target/debug/libnero_%.so:
	cd plugins/$* && cargo build
	@sed -i 's@^/.*\.\./target@target@' target/debug/libnero_$*.d

-include $(wildcard target/release/*.d)
-include $(wildcard target/debug/*.d)
