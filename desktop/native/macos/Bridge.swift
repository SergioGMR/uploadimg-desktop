import AppKit
import UserNotifications
import ImageIO
import Darwin

// Web brand image glyph: Icon.astro's 24 × 24 viewBox, stroke 1.75,
// rounded caps/joins. Flipped coordinates preserve the SVG's top-down geometry.
func menuBarIcon() -> NSImage {
    let image = NSImage(size: NSSize(width: 18, height: 18), flipped: true) { _ in
        NSGraphicsContext.saveGraphicsState()
        defer { NSGraphicsContext.restoreGraphicsState() }
        let scale = NSAffineTransform(); scale.scale(by: 0.75); scale.concat()
        NSColor.black.setStroke()
        let outline = NSBezierPath(roundedRect: NSRect(x: 3, y: 3, width: 18, height: 18), xRadius: 2, yRadius: 2)
        let landscape = NSBezierPath()
        landscape.move(to: NSPoint(x: 3, y: 16))
        for point in [NSPoint(x: 8, y: 11), NSPoint(x: 13, y: 16), NSPoint(x: 16, y: 13), NSPoint(x: 21, y: 18)] {
            landscape.line(to: point)
        }
        let dot = NSBezierPath(); dot.move(to: NSPoint(x: 15, y: 7)); dot.line(to: NSPoint(x: 15.01, y: 7))
        for path in [outline, landscape, dot] {
            path.lineWidth = 1.75; path.lineCapStyle = .round; path.lineJoinStyle = .round; path.stroke()
        }
        return true
    }
    image.isTemplate = true
    return image
}

// Buffers belong to Swift and are valid only for the synchronous callback.
// Rust copies bounded image/thumbnail/text data before returning; no pointers survive.
public typealias NativeCallback = @convention(c) (
    Int32, UInt64, UInt64, Int32, UInt32, UInt32,
    UnsafePointer<UInt8>?, Int, UnsafePointer<UInt8>?, Int,
    UnsafePointer<UInt8>?, Int
) -> Void

final class SystemBoard: ImagePasteboard {
    private let board = NSPasteboard.general
    var denied: Bool {
        if #available(macOS 15.4, *) { return board.accessBehavior == .alwaysDeny }
        return true
    }
    var changeCount: Int { board.changeCount }
    var imageTypes: [String]? { board.types?.map(\.rawValue) }
    func imageData(type: String) -> Data? { board.data(forType: NSPasteboard.PasteboardType(type)) }
}

final class NativeBridge: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NativeBridge()
    var callback: NativeCallback?
    private let worker = DispatchQueue(label: "uploadimg.desktop.single-decoder", qos: .userInitiated)
    private let clipboard = ClipboardMonitor()
    private var board: ImagePasteboard?
    private let boardFactory: () -> ImagePasteboard
    private let sendNotification: ((UNNotificationRequest) -> Void)?
    private let removeNotifications: (([String]) -> Void)?
    // Control token belongs exclusively to Rust; native wake changes lifecycle only.
    private var generation: UInt64 = 0
    private var lifecycle: UInt64 = 0
    private var configurationRevision: UInt64 = 0
    private var notificationRevision: UInt64 = 0
    private var liveIDs: Set<String> = []
    private var desiredClipboard = false
    private var paused = false
    private var busy = false
    private var timer: Timer?
    private var tray: NSStatusItem?
    var notificationsAllowed = false
    private(set) var notificationIDs: Set<String> = []

    init(boardFactory: @escaping () -> ImagePasteboard = { SystemBoard() },
         sendNotification: ((UNNotificationRequest) -> Void)? = nil,
         removeNotifications: (([String]) -> Void)? = nil) {
        self.boardFactory = boardFactory
        self.sendNotification = sendNotification
        self.removeNotifications = removeNotifications
        super.init()
    }

    func start(_ callback: @escaping NativeCallback) {
        self.callback = callback
        let center = UNUserNotificationCenter.current(); center.delegate = self
        let candidate = UNNotificationCategory(identifier: "CANDIDATE", actions: [
            UNNotificationAction(identifier: "SHARE", title: "Compartir (simulado)", options: [.foreground]),
            UNNotificationAction(identifier: "IGNORE", title: "Ignorar", options: [])],
            intentIdentifiers: [], options: [.customDismissAction])
        let result = UNNotificationCategory(identifier: "RESULT", actions: [
            UNNotificationAction(identifier: "COPY", title: "Copiar enlace ficticio", options: []),
            UNNotificationAction(identifier: "OPEN", title: "Abrir enlace ficticio", options: [.foreground])],
            intentIdentifiers: [], options: [.customDismissAction])
        center.setNotificationCategories([candidate, result])
        timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in self?.poll() }
        NSWorkspace.shared.notificationCenter.addObserver(self, selector: #selector(wake), name: NSWorkspace.didWakeNotification, object: nil)
        tray = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        tray?.button?.title = ""
        tray?.button?.image = menuBarIcon()
        tray?.button?.imagePosition = .imageOnly
        tray?.button?.toolTip = "Uploadimg"
        tray?.button?.setAccessibilityLabel("Uploadimg")
        updateTray()
    }

    func configure(enabled: Bool, paused: Bool, generation: UInt64, userChange: Bool, revision: UInt64) {
        guard revision > configurationRevision else { return }
        configurationRevision = revision
        lifecycle += 1
        applyConfiguration(enabled: enabled, paused: paused, generation: generation, userChange: userChange)
        emit(kind: 5, code: 1)
    }
    private func applyConfiguration(enabled: Bool, paused: Bool, generation: UInt64, userChange: Bool) {
        self.generation = generation; self.desiredClipboard = enabled; self.paused = paused
        clipboard.disable(); board = nil
        if userChange { UserDefaults.standard.set(enabled, forKey: "clipboardIntent") }
        if enabled && !paused {
            let board = boardFactory()
            if clipboard.enable(board) { self.board = board }
            else { desiredClipboard = false; emit(kind: 3, code: 0) }
        }
        updateTray()
    }
    func poll() {
        guard !paused, !busy, clipboard.enabled, let board else { return }
        observe(clipboard.poll(board), source: 1)
    }
    func paste() {
        guard !paused, !busy else { emit(kind: 2, code: 1); return }
        observe(clipboard.explicitPaste(SystemBoard()), source: 2)
    }
    func synthetic() {
        guard !paused, !busy else { emit(kind: 2, code: 1); return }
        if let bytes = try? SyntheticImage.make(width: 640, height: 360, srgb: true) { prepare(bytes, source: 0) }
    }
    private func observe(_ observation: ClipboardObservation, source: Int32) {
        switch observation {
        case let .image(bytes): prepare(bytes, source: source)
        case .denied:
            desiredClipboard = false; clipboard.disable(); board = nil
            UserDefaults.standard.set(false, forKey: "clipboardIntent")
            emit(kind: 3, code: 0)
        case .tooLarge: emit(kind: 2, code: 2)
        case .unsupported: if source == 2 { emit(kind: 2, code: 3) }
        case .unchanged: break
        }
    }
    private func prepare(_ bytes: Data, source: Int32) {
        busy = true
        let epoch = generation
        let cycle = lifecycle
        worker.async { [weak self] in
            guard let self else { return }
            let prepared: Result<(PreparedImage, Data), Error> = Result {
                try autoreleasepool {
                    let image = try ImageDecoder.prepare(bytes)
                    let thumbnail = NSMutableData()
                    guard let output = CGImageDestinationCreateWithData(thumbnail, "public.png" as CFString, 1, nil) else { throw ProbeError.invalidImage }
                    CGImageDestinationAddImage(output, image.preview, nil)
                    guard CGImageDestinationFinalize(output), thumbnail.length <= 1_100_000 else { throw ProbeError.tooLarge }
                    return (image, thumbnail as Data)
                }
            }
            DispatchQueue.main.async {
                self.busy = false
                guard self.acceptsImage(generation: epoch, lifecycle: cycle), !self.paused else { return }
                switch prepared {
                case let .success((prepared, thumbnail)):
                    self.emitImage(prepared.image, thumbnail: thumbnail, source: source, generation: epoch, lifecycle: cycle)
                case .failure: self.emit(kind: 2, code: 4)
                }
            }
        }
    }

    // This copies no image into a disk file, notification or WebView. The Rust
    // callback owns admission and copies the in-memory immutable snapshot.
    func emitImage(_ image: FrozenImage, thumbnail: Data, source: Int32, generation: UInt64, lifecycle: UInt64) {
        guard let callback else { return }
        let digest = Data(image.digest.utf8)
        image.bytes.withUnsafeBytes { original in
            thumbnail.withUnsafeBytes { preview in
                digest.withUnsafeBytes { text in
                    callback(1, generation, lifecycle, source, UInt32(image.width), UInt32(image.height),
                        original.bindMemory(to: UInt8.self).baseAddress, original.count,
                        preview.bindMemory(to: UInt8.self).baseAddress, preview.count,
                        text.bindMemory(to: UInt8.self).baseAddress, text.count)
                }
            }
        }
    }
    func emit(kind: Int32, code: Int32, text: String = "") {
        let text = Data(text.utf8)
        text.withUnsafeBytes { bytes in
            callback?(kind, generation, lifecycle, code, 0, 0, nil, 0, nil, 0,
                bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
        }
    }
    func acceptsImage(generation: UInt64, lifecycle: UInt64) -> Bool {
        generation == self.generation && lifecycle == self.lifecycle && !paused
    }
    @objc func wake() {
        lifecycle += 1
        applyConfiguration(enabled: desiredClipboard, paused: paused, generation: generation, userChange: false)
        emit(kind: 5, code: 0)
    }
    @objc private func show() { emit(kind: 6, code: 7) }
    @objc private func togglePause() { emit(kind: 6, code: 8) }
    @objc private func quit() { emit(kind: 6, code: 9) }
    private func updateTray() {
        let menu = NSMenu()
        for (title, action) in [("Abrir Uploadimg Desktop", #selector(show)),
                                 (paused ? "Reanudar" : "Pausar", #selector(togglePause)), ("Salir", #selector(quit))] {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.target = self; menu.addItem(item)
        }
        tray?.menu = menu
    }

    func requestNotifications() {
        emit(kind: 4, code: 2)
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { [weak self] allowed, _ in
            DispatchQueue.main.async {
                self?.notificationsAllowed = allowed
                self?.emit(kind: 4, code: allowed ? 1 : 0)
            }
        }
    }
    func notify(id: String, result: Bool, revision: UInt64) {
        let identifier = (result ? "result." : "candidate.") + id
        guard revision <= notificationRevision, liveIDs.contains(identifier),
              notificationsAllowed, validHex(id, length: result ? 48 : 32),
              !notificationIDs.contains(identifier) else { return }
        let content = UNMutableNotificationContent()
        content.title = result ? "Enlace ficticio listo · simulación local" : "Imagen lista para compartir · simulación local"
        content.body = result ? "https://example.invalid/i/\(id)" : "Abrir muestra la vista previa. Compartir solo simula una subida."
        content.categoryIdentifier = result ? "RESULT" : "CANDIDATE"
        content.threadIdentifier = result ? "uploadimg-desktop-results" : "uploadimg-desktop-candidates"
        notificationIDs.insert(identifier)
        let request = UNNotificationRequest(identifier: identifier, content: content, trigger: nil)
        if let sendNotification { sendNotification(request); return }
        UNUserNotificationCenter.current().add(request) { [weak self] error in
            if error != nil { DispatchQueue.main.async { self?.emit(kind: 2, code: 5) } }
        }
    }
    func sync(ids: [String], revision: UInt64) {
        guard revision > notificationRevision, ids.count <= 23, ids.allSatisfy({ identifier in
            if identifier.hasPrefix("candidate.") { return validHex(String(identifier.dropFirst(10)), length: 32) }
            if identifier.hasPrefix("result.") { return validHex(String(identifier.dropFirst(7)), length: 48) }
            return false
        }) else { return }
        notificationRevision = revision
        let live = Set(ids)
        liveIDs = live
        let stale = notificationIDs.subtracting(live)
        if let removeNotifications { removeNotifications(Array(stale)) }
        else {
            UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: Array(stale))
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: Array(stale))
        }
        notificationIDs.subtract(stale)
    }
    func resultOperation(id: String, action: Int32) {
        guard liveIDs.contains("result." + id), validHex(id, length: 48), let url = URL(string: "https://example.invalid/i/\(id)") else { return }
        if action == 5 {
            NSPasteboard.general.clearContents(); NSPasteboard.general.writeObjects([url.absoluteString as NSString])
        } else if action == 6 { NSWorkspace.shared.open(url) }
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification,
                                withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.banner, .list])
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
                                withCompletionHandler completionHandler: @escaping () -> Void) {
        DispatchQueue.main.async {
            defer { completionHandler() }
            let request = response.notification.request
            guard self.notificationIDs.contains(request.identifier) else { return }
            let candidate = request.identifier.hasPrefix("candidate.") && request.content.categoryIdentifier == "CANDIDATE"
            let result = request.identifier.hasPrefix("result.") && request.content.categoryIdentifier == "RESULT"
            guard candidate || result else { return }
            let id = String(request.identifier.dropFirst(candidate ? 10 : 7))
            guard validHex(id, length: candidate ? 32 : 48) else { return }
            let code: Int32
            switch response.actionIdentifier {
            case UNNotificationDefaultActionIdentifier: code = candidate ? 1 : 4
            case UNNotificationDismissActionIdentifier: code = candidate ? 3 : 0
            case "SHARE": code = candidate ? 2 : 0
            case "IGNORE": code = candidate ? 3 : 0
            case "COPY": code = result ? 5 : 0
            case "OPEN": code = result ? 6 : 0
            default: code = 0
            }
            if code != 0 { self.emit(kind: 6, code: code, text: id) }
        }
    }
}
func validHex(_ value: String, length: Int) -> Bool {
    value.utf8.count == length && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
}

@_cdecl("uig_start")
public func uigStart(_ callback: @escaping NativeCallback) {
    DispatchQueue.main.async { NativeBridge.shared.start(callback) }
}
@_cdecl("uig_configure")
public func uigConfigure(_ enabled: Int32, _ paused: Int32, _ generation: UInt64, _ userChange: Int32, _ revision: UInt64) {
    let apply = { NativeBridge.shared.configure(enabled: enabled != 0, paused: paused != 0, generation: generation, userChange: userChange != 0, revision: revision) }
    // Rust never holds its state/queue locks while calling this control barrier.
    // Returning from disable/pause means subsequent timer reads are stopped.
    if Thread.isMainThread { apply() } else { DispatchQueue.main.sync(execute: apply) }
}
@_cdecl("uig_generate")
public func uigGenerate() { DispatchQueue.main.async { NativeBridge.shared.synthetic() } }
@_cdecl("uig_paste")
public func uigPaste() { DispatchQueue.main.async { NativeBridge.shared.paste() } }
@_cdecl("uig_request_notifications")
public func uigRequestNotifications() { DispatchQueue.main.async { NativeBridge.shared.requestNotifications() } }
@_cdecl("uig_notify")
public func uigNotify(_ pointer: UnsafePointer<CChar>, _ result: Int32, _ revision: UInt64) {
    let id = String(cString: pointer)
    DispatchQueue.main.async { NativeBridge.shared.notify(id: id, result: result != 0, revision: revision) }
}
@_cdecl("uig_sync_ids")
public func uigSyncIDs(_ pointer: UnsafePointer<CChar>, _ revision: UInt64) {
    let text = String(cString: pointer)
    guard text.utf8.count <= 2048, let bytes = text.data(using: .utf8),
          let ids = try? JSONSerialization.jsonObject(with: bytes) as? [String] else { return }
    DispatchQueue.main.async { NativeBridge.shared.sync(ids: ids, revision: revision) }
}
@_cdecl("uig_result_operation")
public func uigResultOperation(_ pointer: UnsafePointer<CChar>, _ action: Int32) {
    let id = String(cString: pointer)
    DispatchQueue.main.async { NativeBridge.shared.resultOperation(id: id, action: action) }
}
@_cdecl("uig_monotonic_millis")
public func uigMonotonicMillis() -> UInt64 {
    var scale = mach_timebase_info_data_t(); mach_timebase_info(&scale)
    return UInt64(Double(mach_continuous_time()) * Double(scale.numer) / Double(scale.denom) / 1_000_000)
}
