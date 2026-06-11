import Cocoa
import Carbon.HIToolbox

// keyCodes that should reset the syllable buffer (navigation / editing)
private let resetKeyCodes: Set<CGKeyCode> = [
    36,  // Return
    48,  // Tab
    53,  // Escape
    115, // Home
    116, // Page Up
    117, // Forward Delete
    119, // End
    121, // Page Down
    123, // Arrow Left
    124, // Arrow Right
    125, // Arrow Down
    126, // Arrow Up
]

class KeyboardInterceptor {
    private var eventTap: CFMachPort?
    private var engine: UnsafeMutableRawPointer?
    private let magic: Int64 = 0x504B544B // "PKTK"
    private let candidateWindow = CandidateWindow()

    /// What the IME has caused to appear on screen for the syllable being composed.
    /// Used to compute deltas so we avoid injecting backspace events that Chrome
    /// and some Electron apps silently discard.
    private var screenCandidate: String = ""

    init() {
        let preset = Settings.shared.inputMethod
        engine = pktkey_engine_new(preset)
        // Keep engine in sync when settings change from the settings window
        NotificationCenter.default.addObserver(
            forName: .pktkeySettingsChanged,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            guard let self else { return }
            let newPreset = Settings.shared.inputMethod
            self.reloadPreset(newPreset)
            if !Settings.shared.isVietnamese {
                // Sync engine mode with the settings toggle
                let modePtr = pktkey_get_mode(self.engine)
                let mode = modePtr.map { String(cString: $0) } ?? "vi"
                pktkey_free_string(modePtr)
                if mode == "vi" { pktkey_toggle_mode(self.engine) }
            }
            (NSApp.delegate as? AppDelegate)?.updateStatusIconMode(
                Settings.shared.isVietnamese ? "vi" : "en"
            )
        }
    }

    /// Recreate the engine with a new preset without interrupting the session.
    func reloadPreset(_ preset: String) {
        pktkey_reset_buffer(engine)
        pktkey_engine_free(engine)
        engine = pktkey_engine_new(preset)
        candidateWindow.hide()
    }

    deinit {
        if let tap = eventTap { CGEvent.tapEnable(tap: tap, enable: false) }
        pktkey_engine_free(engine)
    }

    func resetBuffer() {
        pktkey_reset_buffer(engine)
        screenCandidate = ""
        candidateWindow.hide()
    }

    func toggleMode() {
        pktkey_toggle_mode(engine)
        screenCandidate = ""
        candidateWindow.hide()
        let modePtr = pktkey_get_mode(engine)
        let mode = modePtr.map { String(cString: $0) } ?? "vi"
        pktkey_free_string(modePtr)
        Settings.shared.isVietnamese = (mode == "vi")
        (NSApp.delegate as? AppDelegate)?.updateStatusIconMode(mode)
    }

    func start() {
        let mask: CGEventMask = (1 << CGEventType.keyDown.rawValue)
                              | (1 << CGEventType.leftMouseDown.rawValue)
        let refcon = Unmanaged.passUnretained(self).toOpaque()
        let cb: CGEventTapCallBack = { _, _, ev, rc in
            guard let rc else { return Unmanaged.passRetained(ev) }
            return Unmanaged<KeyboardInterceptor>.fromOpaque(rc)
                .takeUnretainedValue()
                .handle(event: ev)
        }
        guard let tap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap,
            eventsOfInterest: mask,
            callback: cb,
            userInfo: refcon
        ) else {
            print("PKTKey: CGEventTap failed — grant Accessibility permission")
            return
        }
        let src = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), src, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        eventTap = tap
        print("PKTKey: tap active, Ctrl+Space to toggle Vi/En")
    }

    // MARK: - Event handler

    private func handle(event: CGEvent) -> Unmanaged<CGEvent>? {
        // Pass through events we generated
        if event.getIntegerValueField(.eventSourceUserData) == magic {
            return Unmanaged.passRetained(event)
        }

        // Mouse click: cursor/selection changed externally — buffer state is now stale.
        // Reset so the next backspace passes through and lets the app handle the selection.
        if event.type == .leftMouseDown {
            pktkey_reset_buffer(engine)
            screenCandidate = ""
            candidateWindow.hide()
            return Unmanaged.passRetained(event)
        }

        let flags = event.flags
        // Pass Cmd / Option combos through untouched
        if flags.contains(.maskCommand) || flags.contains(.maskAlternate) {
            return Unmanaged.passRetained(event)
        }

        let keyCode = CGKeyCode(event.getIntegerValueField(.keyboardEventKeycode))

        // Ctrl+Space → toggle mode
        if flags.contains(.maskControl) && keyCode == 49 {
            toggleMode()
            return nil
        }

        // Navigation / editing keys: reset buffer, pass through
        if resetKeyCodes.contains(keyCode) {
            pktkey_reset_buffer(engine)
            screenCandidate = ""
            candidateWindow.hide()
            return Unmanaged.passRetained(event)
        }

        // Backspace (keyCode 51)
        if keyCode == 51 {
            candidateWindow.hide()
            var out = PKTKeyOutput()
            guard pktkey_process_backspace(engine, &out) != 0 else {
                return Unmanaged.passRetained(event)
            }
            defer { freeText(&out) }
            if out.output_type == PKTKEY_PASSTHROUGH {
                screenCandidate = ""
                return Unmanaged.passRetained(event)
            }
            let text = out.text.map { String(cString: $0) } ?? ""
            let del = Int(out.delete_back)
            screenCandidate = text   // backspace always uses full replace path
            if del > 0 { postBackspace(del) }
            if !text.isEmpty { postText(text) }
            return nil
        }

        // Get typed character
        var len = 0
        var buf = [UniChar](repeating: 0, count: 4)
        event.keyboardGetUnicodeString(maxStringLength: 4, actualStringLength: &len, unicodeString: &buf)
        guard len > 0 else { return Unmanaged.passRetained(event) }

        // Accept printable ASCII and Vietnamese Unicode (BMP printable chars ≥ 0x20).
        // Non-BMP (surrogate pairs) and control chars are passed through unchanged.
        guard len >= 1, buf[0] >= 0x20 else {
            return Unmanaged.passRetained(event)
        }
        // Surrogates (0xD800–0xDFFF) are not standalone chars — pass through
        guard buf[0] < 0xD800 || buf[0] > 0xDFFF else {
            return Unmanaged.passRetained(event)
        }

        let chars = String(utf16CodeUnits: Array(buf.prefix(len)), count: len)

        // ── Candidate selection: digit keys 1-5 when popup is visible ──────
        if candidateWindow.isVisible, let digit = chars.first,
           let idx = ["1":0,"2":1,"3":2,"4":3,"5":4][String(digit)],
           idx < candidateWindow.candidates.count {
            let selected = candidateWindow.candidates[idx]
            let delCount = Int(pktkey_candidate_len(engine))
            pktkey_reset_buffer(engine)
            candidateWindow.hide()
            screenCandidate = ""
            // passthrough digit + delete word (delCount) + digit (1) + inject selection
            postBackspace(delCount + 1)
            postText(selected)
            return Unmanaged.passRetained(event)
        }

        // Reset screenCandidate on delimiter REGARDLESS of engine output path
        // (Passthrough in English mode, commit in Vietnamese, etc.) so that
        // a stale candidate never causes over-deletion into preceding text.
        let isDelim = " \n\r\t.,!?;:/-()[]{}\"'`@#$%^&*+=<>|\\".contains(chars)
        defer { if isDelim { screenCandidate = "" } }

        var out = PKTKeyOutput()
        let handled = chars.withCString { pktkey_process_key(engine, $0, &out) }
        guard handled != 0 else { return Unmanaged.passRetained(event) }
        defer { freeText(&out) }

        if out.output_type == PKTKEY_PASSTHROUGH {
            // Engine cleared its buffer (invalid tone on vowel) or English mode.
            // Stale screenCandidate would cause over-deletion on the next keystroke.
            screenCandidate = ""
            candidateWindow.hide()
            return Unmanaged.passRetained(event)
        }

        let text = out.text.map { String(cString: $0) } ?? ""
        let del  = Int(out.delete_back)

        // ── Passthrough: engine just appended the typed char ──────────────
        // When the new candidate == what's already on screen + the typed key,
        // no injection is needed — the original key event already carries the
        // right character.  Injecting a copy would cause doubles in Chrome and
        // Electron apps whose renderers receive raw HID events independently of
        // CGEventTap suppression.
        if text == screenCandidate + chars {
            screenCandidate = text
            updateCandidates(near: event)
            return Unmanaged.passRetained(event)
        }

        // ── Replacement: actual conversion (tone / double-char / revert) ──
        // Always pass the original event through so the renderer sees it once.
        // Then inject the minimal delta: compare (screenCandidate + chars) with
        // the new text, delete only the non-matching suffix, and insert the new
        // suffix.  This avoids over-deleting into preceding words/spaces — the
        // root cause of bugs in Chrome/Safari sandboxed renderers.
        let oldOnScreen = screenCandidate + chars
        screenCandidate = text

        let oldChars = Array(oldOnScreen)
        let newChars = Array(text)
        var prefix = 0
        while prefix < oldChars.count && prefix < newChars.count && oldChars[prefix] == newChars[prefix] {
            prefix += 1
        }
        let bsDelta = oldChars.count - prefix
        let newSuffix = String(newChars[prefix...])

        if bsDelta > 0 { postBackspace(bsDelta) }
        if !newSuffix.isEmpty { postText(newSuffix) }
        updateCandidates(near: event)
        return Unmanaged.passRetained(event)
    }

    // MARK: - Candidates

    private func updateCandidates(near event: CGEvent) {
        var count: UInt = 0
        guard let arr = pktkey_get_suggestions(engine, &count), count > 0 else {
            candidateWindow.hide()
            return
        }
        defer { pktkey_free_suggestions(arr, count) }
        var words: [String] = []
        for i in 0..<Int(count) {
            if let p = arr[i] { words.append(String(cString: p)) }
        }
        if words.isEmpty {
            candidateWindow.hide()
            return
        }
        // Position near mouse cursor as approximation for text caret
        let mouse = NSEvent.mouseLocation
        DispatchQueue.main.async { [weak self] in
            self?.candidateWindow.show(candidates: words, near: mouse)
        }
    }

    // MARK: - Event injection

    private func postBackspace(_ n: Int) {
        let src = CGEventSource(stateID: .hidSystemState)
        for _ in 0..<n {
            inject(CGEvent(keyboardEventSource: src, virtualKey: 51, keyDown: true))
            inject(CGEvent(keyboardEventSource: src, virtualKey: 51, keyDown: false))
        }
    }

    private func postText(_ text: String) {
        let src = CGEventSource(stateID: .hidSystemState)
        var utf16 = Array(text.utf16)
        let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true)
        down?.keyboardSetUnicodeString(stringLength: utf16.count, unicodeString: &utf16)
        inject(down)
        let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false)
        up?.keyboardSetUnicodeString(stringLength: utf16.count, unicodeString: &utf16)
        inject(up)
    }

    private func inject(_ event: CGEvent?) {
        guard let ev = event else { return }
        ev.setIntegerValueField(.eventSourceUserData, value: magic)
        ev.post(tap: .cgSessionEventTap)
    }

    private func freeText(_ out: inout PKTKeyOutput) {
        if out.text != nil { pktkey_free_string(out.text); out.text = nil }
    }
}
