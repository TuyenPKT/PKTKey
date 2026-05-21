import Cocoa

/// Floating candidate window shown when the engine has word suggestions.
/// Up to 5 rows; user selects with digit keys 1–5.
class CandidateWindow: NSPanel {
    private var rows: [NSTextField] = []
    private(set) var candidates: [String] = []

    // ── Init ──────────────────────────────────────────────────────────────

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 180, height: 0),
            styleMask:   [.borderless, .nonactivatingPanel],
            backing:     .buffered,
            defer:       false
        )
        level            = .popUpMenu
        isOpaque         = false
        backgroundColor  = .clear
        hasShadow        = true
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]

        let bg = NSView(frame: contentView!.bounds)
        bg.autoresizingMask   = [.width, .height]
        bg.wantsLayer         = true
        bg.layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.95).cgColor
        bg.layer?.cornerRadius    = 8
        bg.layer?.borderWidth     = 0.5
        bg.layer?.borderColor     = NSColor.separatorColor.cgColor
        contentView!.addSubview(bg)

        for i in 0..<5 {
            let tf = NSTextField(frame: .zero)
            tf.isEditable   = false
            tf.isBordered   = false
            tf.drawsBackground = false
            tf.font         = .systemFont(ofSize: 14)
            tf.textColor    = .labelColor
            tf.lineBreakMode = .byTruncatingTail
            tf.tag          = i
            bg.addSubview(tf)
            rows.append(tf)
        }
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Show the window near `screenPoint` with `candidates`.
    func show(candidates: [String], near screenPoint: NSPoint) {
        self.candidates = candidates
        let visible = min(candidates.count, 5)

        let rowH: CGFloat  = 24
        let padX: CGFloat  = 10
        let padY: CGFloat  = 6
        let width: CGFloat = 200
        let totalH         = CGFloat(visible) * rowH + padY * 2

        for (i, tf) in rows.enumerated() {
            if i < visible {
                tf.stringValue  = "\(i + 1). \(candidates[i])"
                tf.frame        = NSRect(x: padX, y: padY + CGFloat(visible - 1 - i) * rowH,
                                         width: width - padX * 2, height: rowH)
                tf.isHidden     = false
            } else {
                tf.isHidden = true
            }
        }

        let frame = NSRect(x: screenPoint.x + 4,
                           y: screenPoint.y - totalH - 4,
                           width: width, height: totalH)
        setFrame(frame, display: false)
        contentView?.subviews.first?.frame = NSRect(x: 0, y: 0, width: width, height: totalH)

        orderFront(nil)
    }

    func hide() {
        guard isVisible else { return }
        orderOut(nil)
        candidates = []
    }
}
