import Foundation
import ServiceManagement

/// Persisted user preferences backed by UserDefaults.
final class Settings {
    static let shared = Settings()
    private init() {}

    // MARK: - Keys
    private enum Key {
        static let inputMethod  = "inputMethod"
        static let isVietnamese = "isVietnamese"
        static let autoRevert   = "autoRevert"
        static let showNotif    = "showNotif"
        static let quickType    = "quickType"
    }

    // MARK: - Properties

    /// "telex" | "vni" | "viqr"
    var inputMethod: String {
        get { UserDefaults.standard.string(forKey: Key.inputMethod) ?? "telex" }
        set { UserDefaults.standard.set(newValue, forKey: Key.inputMethod) }
    }

    /// Whether Vietnamese input mode is active
    var isVietnamese: Bool {
        get { UserDefaults.standard.object(forKey: Key.isVietnamese) as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: Key.isVietnamese) }
    }

    /// Auto-revert invalid syllables to raw keystrokes
    var autoRevert: Bool {
        get { UserDefaults.standard.object(forKey: Key.autoRevert) as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: Key.autoRevert) }
    }

    /// Show notification on mode toggle
    var showNotif: Bool {
        get { UserDefaults.standard.object(forKey: Key.showNotif) as? Bool ?? false }
        set { UserDefaults.standard.set(newValue, forKey: Key.showNotif) }
    }

    /// Enable quick-type abbreviations (gõ tắt)
    var quickType: Bool {
        get { UserDefaults.standard.object(forKey: Key.quickType) as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: Key.quickType) }
    }

    /// Launch PKTKey automatically at login (backed by SMAppService, not UserDefaults)
    var launchAtLogin: Bool {
        get { SMAppService.mainApp.status == .enabled }
        set {
            do {
                if newValue {
                    try SMAppService.mainApp.register()
                } else {
                    try SMAppService.mainApp.unregister()
                }
                print("PKTKey: launchAtLogin=\(newValue) status=\(SMAppService.mainApp.status)")
            } catch {
                print("PKTKey: launchAtLogin error — \(error.localizedDescription)")
            }
        }
    }
}
