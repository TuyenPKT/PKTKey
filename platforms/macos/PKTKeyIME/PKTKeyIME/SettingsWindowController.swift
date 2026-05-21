import Cocoa

final class SettingsWindowController: NSWindowController {

    // MARK: - Singleton
    static let shared = SettingsWindowController()

    // MARK: - State
    private var inputMethodPopup  = NSPopUpButton()
    private var toggleKeyPopup    = NSPopUpButton()
    private var viCheckbox        = NSButton()
    private var quickTypeCheckbox = NSButton()
    private var autoRevertCheckbox = NSButton()
    private var notifCheckbox     = NSButton()
    private var modeButton        = NSButton()
    private var tryTextField      = NSTextField()

    // MARK: - Init
    private init() {
        let w = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 780, height: 520),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        w.title = "PKTKey"
        w.center()
        w.isReleasedWhenClosed = false
        super.init(window: w)
        buildUI()
        loadSettings()
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func showAndFocus() {
        loadSettings()
        window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    // MARK: - Build UI

    private func buildUI() {
        guard let content = window?.contentView else { return }
        content.wantsLayer = true
        content.layer?.backgroundColor = NSColor(red: 0.97, green: 0.97, blue: 0.98, alpha: 1).cgColor

        // ── Header ────────────────────────────────────────────────────────────
        let header = makeHeaderView()
        header.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(header)

        // ── Divider ───────────────────────────────────────────────────────────
        let divider = NSBox()
        divider.boxType = .separator
        divider.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(divider)

        // ── Left panel (settings form) ────────────────────────────────────────
        let left = makeLeftPanel()
        left.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(left)

        // ── Right panel (feature cards) ───────────────────────────────────────
        let right = makeRightPanel()
        right.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(right)

        // ── Bottom divider ────────────────────────────────────────────────────
        let divider2 = NSBox()
        divider2.boxType = .separator
        divider2.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(divider2)

        // ── Gõ thử ────────────────────────────────────────────────────────────
        let tryRow = makeTryRow()
        tryRow.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(tryRow)

        // ── Bottom buttons ────────────────────────────────────────────────────
        let buttons = makeButtonRow()
        buttons.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(buttons)

        NSLayoutConstraint.activate([
            // Header
            header.topAnchor.constraint(equalTo: content.topAnchor),
            header.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            header.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            header.heightAnchor.constraint(equalToConstant: 70),

            divider.topAnchor.constraint(equalTo: header.bottomAnchor),
            divider.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            divider.trailingAnchor.constraint(equalTo: content.trailingAnchor),

            // Panels
            left.topAnchor.constraint(equalTo: divider.bottomAnchor, constant: 16),
            left.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            left.widthAnchor.constraint(equalToConstant: 360),

            right.topAnchor.constraint(equalTo: divider.bottomAnchor, constant: 16),
            right.leadingAnchor.constraint(equalTo: left.trailingAnchor, constant: 16),
            right.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),

            // Gõ thử
            divider2.topAnchor.constraint(equalTo: left.bottomAnchor, constant: 16),
            divider2.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            divider2.trailingAnchor.constraint(equalTo: content.trailingAnchor),

            tryRow.topAnchor.constraint(equalTo: divider2.bottomAnchor, constant: 12),
            tryRow.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            tryRow.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),

            // Bottom buttons
            buttons.topAnchor.constraint(equalTo: tryRow.bottomAnchor, constant: 12),
            buttons.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            buttons.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),
            buttons.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -16),
        ])
    }

    // MARK: - Header

    private func makeHeaderView() -> NSView {
        let v = NSView()
        v.wantsLayer = true
        v.layer?.backgroundColor = NSColor.white.cgColor

        let logo = NSTextField(labelWithString: "PKT")
        logo.font = .boldSystemFont(ofSize: 22)
        logo.textColor = NSColor(red: 0.13, green: 0.37, blue: 0.93, alpha: 1)

        let logoKey = NSTextField(labelWithString: "Key")
        logoKey.font = .boldSystemFont(ofSize: 22)
        logoKey.textColor = .labelColor

        let title = NSTextField(labelWithString: "Vietnamese Input Method")
        title.font = .systemFont(ofSize: 14, weight: .semibold)

        let subtitle = NSTextField(labelWithString: "Dựa trên UniKey — Đơn giản hơn, Gọn nhẹ hơn.")
        subtitle.font = .systemFont(ofSize: 11)
        subtitle.textColor = .secondaryLabelColor

        let version = NSTextField(labelWithString: "v1.0.0")
        version.font = .systemFont(ofSize: 11)
        version.textColor = .tertiaryLabelColor
        version.alignment = .right

        for v2 in [logo, logoKey, title, subtitle, version] {
            v2.translatesAutoresizingMaskIntoConstraints = false
            v.addSubview(v2)
        }

        NSLayoutConstraint.activate([
            logo.leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: 24),
            logo.centerYAnchor.constraint(equalTo: v.centerYAnchor),

            logoKey.leadingAnchor.constraint(equalTo: logo.trailingAnchor),
            logoKey.centerYAnchor.constraint(equalTo: logo.centerYAnchor),

            title.leadingAnchor.constraint(equalTo: logoKey.trailingAnchor, constant: 16),
            title.topAnchor.constraint(equalTo: v.topAnchor, constant: 16),

            subtitle.leadingAnchor.constraint(equalTo: title.leadingAnchor),
            subtitle.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 2),

            version.trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -24),
            version.topAnchor.constraint(equalTo: v.topAnchor, constant: 16),
        ])
        return v
    }

    // MARK: - Left panel

    private func makeLeftPanel() -> NSView {
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10

        // Form rows
        stack.addArrangedSubview(makeFormRow(label: "Bảng mã", control: makeEncodingPopup()))
        stack.addArrangedSubview(makeFormRow(label: "Kiểu gõ", control: inputMethodPopup))
        stack.addArrangedSubview(makeFormRow(label: "Phím chuyển", control: toggleKeyPopup))

        stack.addArrangedSubview(makeSpacer(8))

        // Checkboxes
        let optLabel = NSTextField(labelWithString: "Tùy chọn")
        optLabel.font = .systemFont(ofSize: 12, weight: .semibold)
        optLabel.textColor = .secondaryLabelColor
        stack.addArrangedSubview(optLabel)

        viCheckbox        = makeCheckbox("Gõ tiếng Việt",     action: #selector(viCheckboxChanged))
        quickTypeCheckbox = makeCheckbox("Gõ tắt",            action: #selector(quickTypeChanged))
        autoRevertCheckbox = makeCheckbox("Tự động khôi phục phím với từ sai", action: #selector(autoRevertChanged))
        notifCheckbox     = makeCheckbox("Hiện thông báo",    action: #selector(notifChanged))

        for cb in [viCheckbox, quickTypeCheckbox, autoRevertCheckbox, notifCheckbox] {
            stack.addArrangedSubview(cb)
        }

        // Populate popups
        inputMethodPopup.removeAllItems()
        inputMethodPopup.addItems(withTitles: ["Telex Codex", "Telex", "VNI", "VIQR"])
        inputMethodPopup.target = self
        inputMethodPopup.action = #selector(inputMethodChanged)

        toggleKeyPopup.removeAllItems()
        toggleKeyPopup.addItems(withTitles: ["Ctrl + Space", "Ctrl + Shift", "Alt + Z"])

        return stack
    }

    private func makeEncodingPopup() -> NSPopUpButton {
        let p = NSPopUpButton()
        p.addItem(withTitle: "Unicode")
        p.widthAnchor.constraint(equalToConstant: 140).isActive = true
        return p
    }

    private func makeFormRow(label: String, control: NSView) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 8

        let lbl = NSTextField(labelWithString: label + ":")
        lbl.font = .systemFont(ofSize: 13)
        lbl.widthAnchor.constraint(equalToConstant: 110).isActive = true
        lbl.alignment = .right

        if let popup = control as? NSPopUpButton {
            popup.widthAnchor.constraint(equalToConstant: 140).isActive = true
        }

        row.addArrangedSubview(lbl)
        row.addArrangedSubview(control)
        return row
    }

    private func makeCheckbox(_ title: String, action: Selector) -> NSButton {
        let b = NSButton(checkboxWithTitle: title, target: self, action: action)
        b.font = .systemFont(ofSize: 13)
        return b
    }

    private func makeSpacer(_ h: CGFloat) -> NSView {
        let v = NSView()
        v.heightAnchor.constraint(equalToConstant: h).isActive = true
        return v
    }

    // MARK: - Right panel

    private func makeRightPanel() -> NSView {
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12

        let heading = NSTextField(labelWithString: "Đơn giản – Dễ dùng – Nhẹ")
        heading.font = .systemFont(ofSize: 14, weight: .semibold)
        heading.textColor = NSColor(red: 0.13, green: 0.37, blue: 0.93, alpha: 1)
        stack.addArrangedSubview(heading)

        let features: [(String, String, NSColor)] = [
            ("Dễ dùng",            "Giao diện tối giản, mọi thứ bạn cần đều ở một nơi.",                     NSColor(red: 0.13, green: 0.37, blue: 0.93, alpha: 1)),
            ("Nhẹ & Nhanh",        "Tối ưu hiệu năng, khởi động nhanh, chiếm ít tài nguyên.",               NSColor(red: 0.18, green: 0.68, blue: 0.34, alpha: 1)),
            ("Ổn định",            "Kế thừa độ ổn định từ UniKey, hoạt động mượt mà.",                       NSColor(red: 0.38, green: 0.22, blue: 0.72, alpha: 1)),
            ("Dành cho mọi người", "Phù hợp cho học tập, làm việc, lập trình, gõ văn bản hàng ngày.",       NSColor(red: 0.90, green: 0.55, blue: 0.10, alpha: 1)),
        ]

        for (title, desc, color) in features {
            stack.addArrangedSubview(makeFeatureCard(title: title, desc: desc, color: color))
        }

        return stack
    }

    private func makeFeatureCard(title: String, desc: String, color: NSColor) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 10
        row.alignment = .top

        let icon = NSView()
        icon.wantsLayer = true
        icon.layer?.backgroundColor = color.cgColor
        icon.layer?.cornerRadius = 8
        icon.widthAnchor.constraint(equalToConstant: 32).isActive = true
        icon.heightAnchor.constraint(equalToConstant: 32).isActive = true

        let text = NSStackView()
        text.orientation = .vertical
        text.alignment = .leading
        text.spacing = 2

        let t = NSTextField(labelWithString: title)
        t.font = .systemFont(ofSize: 13, weight: .semibold)
        t.textColor = color

        let d = NSTextField(wrappingLabelWithString: desc)
        d.font = .systemFont(ofSize: 11)
        d.textColor = .secondaryLabelColor
        d.preferredMaxLayoutWidth = 240

        text.addArrangedSubview(t)
        text.addArrangedSubview(d)

        row.addArrangedSubview(icon)
        row.addArrangedSubview(text)
        return row
    }

    // MARK: - Gõ thử row

    private func makeTryRow() -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 12
        row.alignment = .centerY

        let label = NSTextField(labelWithString: "Gõ thử:")
        label.font = .systemFont(ofSize: 13, weight: .semibold)
        label.widthAnchor.constraint(equalToConstant: 60).isActive = true

        tryTextField.placeholderString = "Gõ Telex vào đây để thử, ví dụ: tieensg → tiếng"
        tryTextField.font = .systemFont(ofSize: 13)
        tryTextField.bezelStyle = .roundedBezel
        tryTextField.focusRingType = .default

        row.addArrangedSubview(label)
        row.addArrangedSubview(tryTextField)
        tryTextField.widthAnchor.constraint(greaterThanOrEqualToConstant: 300).isActive = true

        return row
    }

    // MARK: - Bottom buttons

    private func makeButtonRow() -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 8

        modeButton = NSButton(title: "Bật tiếng Việt", target: self, action: #selector(toggleMode))
        modeButton.bezelStyle = .rounded
        modeButton.keyEquivalent = ""
        styleAccentButton(modeButton)

        let quickTypeBtn = NSButton(title: "Gõ tắt", target: self, action: #selector(openQuickType))
        quickTypeBtn.bezelStyle = .rounded

        let tableBtn = NSButton(title: "Bảng gõ", target: self, action: #selector(openTable))
        tableBtn.bezelStyle = .rounded

        let spacer = NSView()
        spacer.setContentHuggingPriority(.defaultLow, for: .horizontal)

        let quitBtn = NSButton(title: "Thoát", target: self, action: #selector(quitApp))
        quitBtn.bezelStyle = .rounded

        for v in [modeButton, quickTypeBtn, tableBtn, spacer, quitBtn] {
            row.addArrangedSubview(v)
        }
        return row
    }

    private func styleAccentButton(_ btn: NSButton) {
        btn.contentTintColor = .white
        if #available(macOS 11, *) {
            btn.hasDestructiveAction = false
        }
    }

    // MARK: - Load / Save

    private func loadSettings() {
        let s = Settings.shared

        let methods: [String: Int] = ["telex": 0, "telex_original": 1, "vni": 2, "viqr": 3]
        inputMethodPopup.selectItem(at: methods[s.inputMethod] ?? 0)

        viCheckbox.state        = s.isVietnamese ? .on : .off
        quickTypeCheckbox.state = s.quickType    ? .on : .off
        autoRevertCheckbox.state = s.autoRevert  ? .on : .off
        notifCheckbox.state     = s.showNotif    ? .on : .off

        updateModeButtonTitle()
    }

    private func updateModeButtonTitle() {
        modeButton.title = Settings.shared.isVietnamese ? "Tắt tiếng Việt" : "Bật tiếng Việt"
    }

    // MARK: - Actions

    @objc private func inputMethodChanged() {
        let map = ["Telex Codex": "telex", "Telex": "telex_original", "VNI": "vni", "VIQR": "viqr"]
        let selected = inputMethodPopup.titleOfSelectedItem ?? "Telex"
        Settings.shared.inputMethod = map[selected] ?? "telex"
        // Notify interceptor to reload engine preset
        NotificationCenter.default.post(name: .pktkeySettingsChanged, object: nil)
    }

    @objc private func viCheckboxChanged() {
        Settings.shared.isVietnamese = viCheckbox.state == .on
        updateModeButtonTitle()
        NotificationCenter.default.post(name: .pktkeySettingsChanged, object: nil)
    }

    @objc private func quickTypeChanged() {
        Settings.shared.quickType = quickTypeCheckbox.state == .on
    }

    @objc private func autoRevertChanged() {
        Settings.shared.autoRevert = autoRevertCheckbox.state == .on
    }

    @objc private func notifChanged() {
        Settings.shared.showNotif = notifCheckbox.state == .on
    }

    @objc private func toggleMode() {
        Settings.shared.isVietnamese.toggle()
        loadSettings()
        NotificationCenter.default.post(name: .pktkeySettingsChanged, object: nil)
    }

    @objc private func openQuickType() {
        // TODO: Gõ tắt editor window
    }

    @objc private func openTable() {
        // TODO: Bảng gõ window
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }
}

// MARK: - Notification name

extension Notification.Name {
    static let pktkeySettingsChanged = Notification.Name("PKTKeySettingsChanged")
}
