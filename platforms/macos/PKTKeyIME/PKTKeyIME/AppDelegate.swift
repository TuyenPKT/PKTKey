import Cocoa

// Keep status item alive globally — NSStatusItem can be released in release builds
// if held only as an optional property on a weakly-retained delegate.
private var _globalStatusItem: NSStatusItem?

class AppDelegate: NSObject, NSApplicationDelegate {
    private let interceptor = KeyboardInterceptor()
    private var statusItem: NSStatusItem? {
        get { _globalStatusItem }
        set { _globalStatusItem = newValue }
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        try? "launched\n".write(toFile: "/tmp/pktkey_debug.txt", atomically: true, encoding: .utf8)
        setupStatusBar()
        requestAccessibilityThenStart()
        // Reset buffer whenever the user switches to a different app.
        // Without this the engine keeps the previous syllable and the next
        // keypress in the new app triggers a spurious Replace with delete_back > 0.
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.interceptor.resetBuffer()
        }
    }

    // MARK: - Status bar

    private func setupStatusBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        updateStatusIcon()
        rebuildMenu()
    }

    private func rebuildMenu() {
        let menu = NSMenu()
        // Input method submenu
        let methodItem = NSMenuItem(title: "Kiểu gõ", action: nil, keyEquivalent: "")
        let methodSub = NSMenu()
        for (title, key) in [("Telex Codex", "telex"), ("Telex", "telex_original"), ("VNI", "vni"), ("VIQR", "viqr")] {
            let item = NSMenuItem(title: title, action: #selector(selectInputMethod(_:)), keyEquivalent: "")
            item.representedObject = key
            item.state = Settings.shared.inputMethod == key ? .on : .off
            item.target = self
            methodSub.addItem(item)
        }
        methodItem.submenu = methodSub
        menu.addItem(methodItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Bật/Tắt tiếng Việt  (Ctrl+Space)", action: #selector(toggleMode), keyEquivalent: ""))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Cài đặt…", action: #selector(openSettings), keyEquivalent: ","))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Thoát PKTKey", action: #selector(quitApp), keyEquivalent: "q"))
        statusItem?.menu = menu
    }

    func updateStatusIcon() {
        // Shows "VI" when Vietnamese mode, "EN" when English
        statusItem?.button?.title = "VI"
    }

    func updateStatusIconMode(_ mode: String) {
        statusItem?.button?.title = mode == "vi" ? "VI" : "EN"
    }

    @objc private func toggleMode() {
        interceptor.toggleMode()
        rebuildMenu()
    }

    @objc private func selectInputMethod(_ sender: NSMenuItem) {
        guard let key = sender.representedObject as? String else { return }
        Settings.shared.inputMethod = key
        interceptor.reloadPreset(key)
        rebuildMenu()
        updateStatusIconMode(Settings.shared.isVietnamese ? "vi" : "en")
    }

    @objc private func openSettings() {
        SettingsWindowController.shared.showAndFocus()
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }

    // MARK: - Accessibility

    private func requestAccessibilityThenStart() {
        if AXIsProcessTrusted() {
            interceptor.start()
            return
        }
        let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        AXIsProcessTrustedWithOptions(opts)
        print("PKTKey: waiting for Accessibility permission…")
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            self?.requestAccessibilityThenStart()
        }
    }
}
