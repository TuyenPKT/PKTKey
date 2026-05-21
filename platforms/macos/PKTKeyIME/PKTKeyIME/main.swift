import Cocoa

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = AppDelegate()
app.delegate = delegate
_ = delegate  // keep strong reference alive
app.run()
