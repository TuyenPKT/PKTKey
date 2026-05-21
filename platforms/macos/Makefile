RUST_LIB   := ../../target/release/libpktkey_capi.a
APP_DIR    := PKTKeyIME.app/Contents
BINARY     := $(APP_DIR)/MacOS/PKTKeyIME
INSTALL_DIR := $(HOME)/Library/Input\ Methods

ARCH ?= x86_64
TARGET := $(ARCH)-apple-macos13.0

SWIFT_SRCS := Sources/main.swift Sources/AppDelegate.swift Sources/InputController.swift

SWIFTFLAGS := \
    -module-name PKTKeyIME \
    -import-objc-header PKTKeyBridge.h \
    -target $(TARGET) \
    -framework Cocoa \
    -framework InputMethodKit \
    -Xlinker -force_load -Xlinker $(RUST_LIB) \
    -lc -lm

.PHONY: all rust swift bundle sign install clean

all: bundle sign

rust:
	cd ../.. && cargo build --release -p pktkey-capi

swift: rust
	mkdir -p $(APP_DIR)/MacOS
	swiftc $(SWIFTFLAGS) $(SWIFT_SRCS) -o $(BINARY)

bundle: swift
	mkdir -p $(APP_DIR)/Resources
	cp Resources/Info.plist $(APP_DIR)/

sign: bundle
	codesign --force --deep --sign - $(APP_DIR)/..

install: sign
	mkdir -p $(INSTALL_DIR)
	cp -r PKTKeyIME.app $(INSTALL_DIR)/
	@echo ""
	@echo "Installed. Next steps:"
	@echo "  1. System Settings → Keyboard → Input Sources → +"
	@echo "  2. Find 'PKTKey' under Vietnamese or Other"
	@echo "  3. Ctrl+Space in any app to toggle Vi/En"

uninstall:
	rm -rf $(INSTALL_DIR)/PKTKeyIME.app

clean:
	rm -rf PKTKeyIME.app
