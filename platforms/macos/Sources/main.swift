import Cocoa

// InputMethodKit apps have no NIB, so we set up the delegate manually.
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
