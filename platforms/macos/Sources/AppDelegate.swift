import Cocoa
import InputMethodKit
import Carbon

class AppDelegate: NSObject, NSApplicationDelegate {
    var server: IMKServer!

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard
            let info = Bundle.main.infoDictionary,
            let name = info["InputMethodConnectionName"] as? String,
            let bundleID = Bundle.main.bundleIdentifier
        else {
            fatalError("PKTKeyIME: missing InputMethodConnectionName or CFBundleIdentifier")
        }

        // Register the input source on first run so it appears in System Settings.
        // Safe to call repeatedly — no-op if already registered.
        TISRegisterInputSource(Bundle.main.bundleURL as CFURL)

        // IMKServer registers the IME with the system and spawns InputController
        // for each text client that activates this IME.
        server = IMKServer(name: name, bundleIdentifier: bundleID)
    }
}
