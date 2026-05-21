import Cocoa
import InputMethodKit

class InputController: IMKInputController {

    private var engine: UnsafeMutableRawPointer?   // PKTEngine *
    private var candidate: String = ""

    // MARK: - Lifecycle

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        engine = pktkey_engine_new("telex")
    }

    deinit {
        pktkey_engine_free(engine)
    }

    // MARK: - IMKInputController

    /// Called by macOS for every key event directed at this IME.
    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event,
              let client = sender as? (IMKTextInput & NSObjectProtocol)
        else { return false }

        let mods = event.modifierFlags

        // Ctrl+Space: toggle Vi/En mode
        if mods.contains(.control) && event.keyCode == 49 {
            commitPending(client)
            pktkey_toggle_mode(engine)
            return true
        }

        // Let system handle Cmd/Option shortcuts
        if mods.contains(.command) || mods.contains(.option) {
            return false
        }

        // Backspace (keyCode 51)
        if event.keyCode == 51 {
            return handleBackspace(client)
        }

        guard let chars = event.characters, !chars.isEmpty else { return false }
        return handleCharacters(chars, client: client)
    }

    /// Commit any in-progress candidate when the client asks (e.g. focus change).
    override func commitComposition(_ sender: Any!) {
        guard let client = sender as? (IMKTextInput & NSObjectProtocol) else { return }
        commitPending(client)
    }

    // MARK: - Private helpers

    private func handleBackspace(_ client: IMKTextInput & NSObjectProtocol) -> Bool {
        var out = PKTKeyOutput()
        guard pktkey_process_backspace(engine, &out) != 0 else { return false }
        defer { freeOutput(&out) }

        if out.output_type == PKTKEY_PASSTHROUGH {
            // Buffer was empty — let the system delete the previous committed char
            commitPending(client)
            return false
        }

        let newText = out.text.map { String(cString: $0) } ?? ""
        updateMarked(newText, client: client)
        return true
    }

    private func handleCharacters(_ chars: String, client: IMKTextInput & NSObjectProtocol) -> Bool {
        var out = PKTKeyOutput()
        let handled = chars.withCString { ptr in
            pktkey_process_key(engine, ptr, &out)
        }
        guard handled != 0 else { return false }
        defer { freeOutput(&out) }

        switch out.output_type {

        case PKTKEY_PASSTHROUGH:
            // Engine didn't consume the key — commit candidate, let client handle char
            commitPending(client)
            return false

        case PKTKEY_REPLACE:
            let newText = out.text.map { String(cString: $0) } ?? ""
            if isCommitted(newText) {
                // Ends with delimiter → finalize
                client.insertText(newText, replacementRange: NSRange(location: NSNotFound, length: 0))
                candidate = ""
            } else {
                updateMarked(newText, client: client)
            }
            return true

        case PKTKEY_COMMIT:
            let text = out.text.map { String(cString: $0) } ?? ""
            client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: 0))
            candidate = ""
            return true

        default:
            return false
        }
    }

    /// Update the marked (in-progress) text shown in the client.
    private func updateMarked(_ text: String, client: IMKTextInput & NSObjectProtocol) {
        candidate = text
        if text.isEmpty {
            client.setMarkedText(
                "",
                selectionRange: NSRange(location: 0, length: 0),
                replacementRange: NSRange(location: NSNotFound, length: 0)
            )
        } else {
            let attrs: [NSAttributedString.Key: Any] = [
                .underlineStyle: NSUnderlineStyle.single.rawValue
            ]
            let marked = NSAttributedString(string: text, attributes: attrs)
            client.setMarkedText(
                marked,
                selectionRange: NSRange(location: text.utf16.count, length: 0),
                replacementRange: NSRange(location: NSNotFound, length: 0)
            )
        }
    }

    /// Commit the current candidate as final text (no longer marked).
    private func commitPending(_ client: IMKTextInput & NSObjectProtocol) {
        guard !candidate.isEmpty else { return }
        client.insertText(candidate, replacementRange: NSRange(location: NSNotFound, length: 0))
        candidate = ""
    }

    /// True when `text` ends with a word delimiter — meaning the engine already
    /// committed the syllable plus the delimiter in one Replace.
    private func isCommitted(_ text: String) -> Bool {
        guard let last = text.last else { return false }
        return " \n\r\t.,!?;:".contains(last)
    }

    private func freeOutput(_ out: inout PKTKeyOutput) {
        if out.text != nil {
            pktkey_free_string(out.text)
            out.text = nil
        }
    }
}
