import AppKit
import UserNotifications
import ImageIO
import Darwin

struct Failure: Error { let message: String }
func expect(_ check: @autoclosure () throws -> Bool, _ message: String) throws {
    if !(try check()) { throw Failure(message: message) }
}
var passed = 0; var failed = 0
func test(_ name: String, _ body: () throws -> Void) {
    do { try body(); passed += 1; print("PASS \(name)") }
    catch { failed += 1; print("FAIL \(name): \(error)") }
}
final class FakeBoard: ImagePasteboard {
    var deny = false; var count = 0; var types: [String]? = []; var bytes: Data? = nil
    var accesses: [String] = []
    var denied: Bool { accesses.append("policy"); return deny }
    var changeCount: Int { accesses.append("count"); return count }
    var imageTypes: [String]? { accesses.append("types"); return types }
    func imageData(type: String) -> Data? { accesses.append(type); return bytes }
}
test("source OFF reads nothing, baseline skips existing bytes, failed reads stop") {
    let board = FakeBoard(); let monitor = ClipboardMonitor()
    _ = monitor.poll(board); try expect(board.accesses.isEmpty, "OFF touched pasteboard")
    try expect(monitor.enable(board), "enable missing"); _ = monitor.poll(board)
    try expect(!board.accesses.contains("types"), "baseline read existing image")
    board.types = nil; board.count += 1
    guard case .denied = monitor.poll(board) else { throw Failure(message: "read failure not denied") }
    board.accesses = []; _ = monitor.poll(board)
    try expect(board.accesses.isEmpty && !monitor.enabled, "failure source kept polling")
}
test("image-only types read no strings/HTML/URL/file references") {
    let board = FakeBoard(); let monitor = ClipboardMonitor(); _ = monitor.enable(board)
    board.types = ["public.html", "public.utf8-plain-text", "public.url", "public.file-url", "public.png"]
    board.bytes = try SyntheticImage.make(); board.count += 1
    guard case .image = monitor.poll(board) else { throw Failure(message: "supported image missing") }
    try expect(board.accesses.filter { $0.hasPrefix("public.") } == ["public.png"], "nonimage payload read")
}
test("native PNG/TIFF canonical equality, PNG conversion and JPEG dimensions") {
    let png = try ImageDecoder.prepare(SyntheticImage.make())
    let tiff = try ImageDecoder.prepare(SyntheticImage.make(type: "public.tiff"))
    let jpeg = try ImageDecoder.prepare(SyntheticImage.make(type: "public.jpeg"))
    try expect(png.image.digest == tiff.image.digest, "same pixels did not dedup")
    try expect(tiff.image.bytes.starts(with: [137, 80, 78, 71]), "TIFF not PNG normalized")
    try expect(jpeg.image.width == 64 && jpeg.image.height == 64, "JPEG dimensions")
}
func rejects(_ body: () throws -> Void) throws {
    do { try body() } catch { return }; throw Failure(message: "unsafe image accepted")
}
test("native decoder rejects encoded/decoded/animated overflow and creates bounded preview") {
    try rejects { _ = try ImageDecoder.prepare(Data(repeating: 0, count: 10_000_001)) }
    try rejects { _ = try ImageDecoder.prepare(SyntheticImage.make(width: 8193, height: 1)) }
    try rejects { _ = try ImageDecoder.prepare(SyntheticImage.make(width: 4001, height: 4000)) }
    try rejects { _ = try ImageDecoder.prepare(SyntheticImage.make(type: "com.compuserve.gif", frames: 2)) }
    let prepared = try ImageDecoder.prepare(SyntheticImage.make(width: 1600, height: 800, srgb: true))
    try expect(prepared.preview.width == 512 && prepared.preview.height == 256, "unbounded raster preview")
}
test("private native pasteboard TIFF roundtrip, no general clipboard or permission request") {
    let board = NSPasteboard.withUniqueName(); defer { board.releaseGlobally() }
    final class PrivateBoard: ImagePasteboard {
        let board: NSPasteboard
        init(_ board: NSPasteboard) { self.board = board }
        var denied: Bool { false }
        var changeCount: Int { board.changeCount }
        var imageTypes: [String]? { board.types?.map(\.rawValue) }
        func imageData(type: String) -> Data? { board.data(forType: NSPasteboard.PasteboardType(type)) }
    }
    let wrapped = PrivateBoard(board); let monitor = ClipboardMonitor(); _ = monitor.enable(wrapped)
    board.clearContents(); board.setData(try SyntheticImage.make(type: "public.tiff"), forType: .tiff)
    guard case let .image(bytes) = monitor.poll(wrapped) else { throw Failure(message: "private TIFF missing") }
    try expect(try ImageDecoder.prepare(bytes).image.width == 64, "private TIFF decode")
}
var callbackOriginal = Data(); var callbackThumbnail = Data(); var callbackText = Data(); var callbackMeta: [UInt64] = []
func receiver(_ kind: Int32, _ generation: UInt64, _ lifecycle: UInt64, _ source: Int32, _ width: UInt32, _ height: UInt32,
              _ original: UnsafePointer<UInt8>?, _ originalLen: Int, _ thumbnail: UnsafePointer<UInt8>?, _ thumbnailLen: Int,
              _ text: UnsafePointer<UInt8>?, _ textLen: Int) {
    callbackMeta = [UInt64(kind), generation, lifecycle, UInt64(source), UInt64(width), UInt64(height)]
    if let original { callbackOriginal = Data(bytes: original, count: originalLen) }
    if let thumbnail { callbackThumbnail = Data(bytes: thumbnail, count: thumbnailLen) }
    if let text { callbackText = Data(bytes: text, count: textLen) }
}
test("native FFI emits temporary immutable buffers with explicit source/generation metadata") {
    let prepared = try ImageDecoder.prepare(SyntheticImage.make())
    let thumbnail = NSMutableData()
    guard let output = CGImageDestinationCreateWithData(thumbnail, "public.png" as CFString, 1, nil) else { throw Failure(message: "thumbnail destination") }
    CGImageDestinationAddImage(output, prepared.preview, nil); try expect(CGImageDestinationFinalize(output), "thumbnail encode")
    NativeBridge.shared.callback = receiver
    defer { NativeBridge.shared.callback = nil }
    NativeBridge.shared.emitImage(prepared.image, thumbnail: thumbnail as Data, source: 2, generation: 17, lifecycle: 23)
    try expect(callbackMeta == [1, 17, 23, 2, 64, 64], "FFI metadata mismatch")
    try expect(callbackOriginal == prepared.image.bytes, "FFI original bytes changed")
    try expect(callbackThumbnail.count <= 1_100_000 && callbackThumbnail.starts(with: [137, 80, 78, 71]), "FFI thumbnail cap/type")
    try expect(String(data: callbackText, encoding: .utf8) == prepared.image.digest, "FFI canonical digest mismatch")
    thumbnail.length = 0
    try expect(!callbackThumbnail.isEmpty, "callback retained borrowed thumbnail instead of copy")
}
test("delayed old configuration cannot reactivate clipboard after synchronous OFF") {
    let board = FakeBoard()
    let bridge = NativeBridge(boardFactory: { board })
    let delayedEnable = { bridge.configure(enabled: true, paused: false, generation: 1, userChange: false, revision: 1) }
    bridge.configure(enabled: false, paused: false, generation: 2, userChange: false, revision: 2)
    delayedEnable()
    board.accesses = []; board.count += 1
    bridge.poll()
    try expect(board.accesses.isEmpty, "stale enable resumed native reads after OFF")
}
test("delayed synchronization preserves a newer shared result notification and action identity") {
    let candidate = "candidate." + String(repeating: "a", count: 32)
    let resultID = String(repeating: "b", count: 48)
    let result = "result." + resultID
    var delivered: Set<String> = []
    let bridge = NativeBridge(boardFactory: { FakeBoard() },
        sendNotification: { delivered.insert($0.identifier) },
        removeNotifications: { delivered.subtract($0) })
    bridge.notificationsAllowed = true
    bridge.sync(ids: [candidate], revision: 1)
    let delayedSync = { bridge.sync(ids: [candidate], revision: 1) }
    bridge.sync(ids: [result], revision: 2)
    bridge.notify(id: resultID, result: true, revision: 2)
    delayedSync()
    try expect(delivered == [result] && bridge.notificationIDs.contains(result), "stale synchronization removed result notification/action identity")
}
test("delayed notices survive harmless newer sync but cannot resurrect ignored candidates") {
    let id = String(repeating: "c", count: 32)
    let identity = "candidate." + id
    var delivered: Set<String> = []
    let bridge = NativeBridge(boardFactory: { FakeBoard() },
        sendNotification: { delivered.insert($0.identifier) },
        removeNotifications: { delivered.subtract($0) })
    bridge.notificationsAllowed = true
    bridge.sync(ids: [identity], revision: 1)
    bridge.sync(ids: [identity], revision: 2)
    bridge.notify(id: id, result: false, revision: 1)
    try expect(delivered == [identity], "harmless newer tick lost still-live notification")
    bridge.sync(ids: [], revision: 3)
    bridge.notify(id: id, result: false, revision: 1)
    bridge.sync(ids: [identity], revision: 2)
    try expect(delivered.isEmpty && bridge.notificationIDs.isEmpty, "stale notice/sync resurrected ignored candidate")
}
test("pause rejects old enable and newer resume establishes a fresh read baseline") {
    let board = FakeBoard()
    let bridge = NativeBridge(boardFactory: { board })
    bridge.configure(enabled: true, paused: false, generation: 1, userChange: false, revision: 1)
    bridge.configure(enabled: true, paused: true, generation: 2, userChange: false, revision: 2)
    bridge.configure(enabled: true, paused: false, generation: 1, userChange: false, revision: 1)
    board.accesses = []; board.count += 1; bridge.poll()
    try expect(board.accesses.isEmpty, "old enable bypassed pause")
    bridge.configure(enabled: true, paused: false, generation: 3, userChange: false, revision: 3)
    board.accesses = []; bridge.poll()
    try expect(!board.accesses.contains("types"), "resume retrospectively inspected existing content")
    board.count += 1; bridge.poll()
    try expect(board.accesses.contains("types"), "new content not inspected after valid resume")
}
test("wake/control tokens reject in-flight paste but permit new explicit paste while OFF") {
    let board = FakeBoard()
    let bridge = NativeBridge(boardFactory: { board })
    bridge.callback = receiver
    bridge.configure(enabled: true, paused: false, generation: 1, userChange: false, revision: 1)
    bridge.wake()
    try expect(!bridge.acceptsImage(generation: 1, lifecycle: 1), "pre-wake in-flight image accepted")
    try expect(callbackMeta == [5, 1, 2, 0, 0, 0], "wake changed Rust token instead of lifecycle")
    let oldControl = callbackMeta[1]; let oldLifecycle = callbackMeta[2]
    bridge.configure(enabled: false, paused: false, generation: 2, userChange: false, revision: 2)
    try expect(!bridge.acceptsImage(generation: oldControl, lifecycle: oldLifecycle), "in-flight pre-OFF image accepted")
    try expect(!bridge.acceptsImage(generation: 2, lifecycle: oldLifecycle), "lifecycle fence missing")
    try expect(bridge.acceptsImage(generation: callbackMeta[1], lifecycle: callbackMeta[2]), "new explicit paste OFF would be rejected")
    board.accesses = []; board.count += 1; bridge.poll()
    try expect(board.accesses.isEmpty, "OFF failed after wake")
}

final class UploadFixtureProtocol: URLProtocol {
    static var mode = "ok"
    static var requestCount = 0
    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        Self.requestCount += 1
        if Self.mode == "timeout" { return }
        if Self.mode == "redirect" {
            let next = URLRequest(url: URL(string: "https://evil.invalid/private")!)
            let response = HTTPURLResponse(url: request.url!, statusCode: 307, httpVersion: nil, headerFields: ["Location": next.url!.absoluteString])!
            client?.urlProtocol(self, wasRedirectedTo: next, redirectResponse: response)
            return
        }
        let response = HTTPURLResponse(url: request.url!, statusCode: 201, httpVersion: nil,
            headerFields: ["Content-Type": "application/json"])!
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        let body = Self.mode == "oversize" ? Data(repeating: 65, count: 65_537) : Data("{\"ready\":true}".utf8)
        client?.urlProtocol(self, didLoad: body)
        client?.urlProtocolDidFinishLoading(self)
    }
    override func stopLoading() {}
}
func fixtureUpload(_ mode: String, timeout: Double = 1000) -> (Int32, Data) {
    UploadFixtureProtocol.mode = mode; UploadFixtureProtocol.requestCount = 0
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [UploadFixtureProtocol.self]
    var request = URLRequest(url: URL(string: "https://uploadimg.vercel.app/api/uploads/finalize")!)
    request.httpMethod = "POST"; request.httpBody = Data("{}".utf8)
    return performUpload(request, timeout: timeout, configuration: configuration)
}
test("native upload transport returns bounded fixture response and rejects redirects, overflow and timeout") {
    let (status, body) = fixtureUpload("ok")
    try expect(status == 201 && body == Data("{\"ready\":true}".utf8), "fixture response lost")
    let redirect = fixtureUpload("redirect")
    try expect(redirect.0 == 0 && redirect.1.isEmpty && UploadFixtureProtocol.requestCount == 1, "redirect followed or accepted")
    let overflow = fixtureUpload("oversize")
    try expect(overflow.0 == 0 && overflow.1.isEmpty, "unbounded body accepted")
    let timeout = fixtureUpload("timeout", timeout: 10)
    try expect(timeout.0 == 0 && timeout.1.isEmpty, "timeout accepted")
}
test("real native destination guard admits exact production PUT and rejects near misses") {
    let address = "https://vercel.com/api/blob/?pathname=uploadimg%2Fv1%2Fstaging%2Ftest"
    try expect(uploadDestinationAllowed(address, method: "PUT") != nil, "exact production PUT rejected before transport")
    for invalid in ["http://vercel.com/api/blob/?pathname=x", "https://vercel.com/api/blob?pathname=x",
        "https://vercel.com/api/blob//?pathname=x", "https://vercel.com/%61pi/blob/?pathname=x",
        "https://vercel.com/api/blob/", "https://vercel.com.evil.invalid/api/blob/?pathname=x",
        "https://user@vercel.com/api/blob/?pathname=x", "https://vercel.com:443/api/blob/?pathname=x",
        "https://vercel.com/api/blob/?pathname=x#fragment"] {
        try expect(uploadDestinationAllowed(invalid, method: "PUT") == nil, "near miss accepted: \(invalid)")
    }
    try expect(uploadDestinationAllowed(address, method: "POST") == nil, "wrong PUT method accepted")
    try expect(uploadDestinationAllowed("https://uploadimg.vercel.app/api/uploads/finalize", method: "POST") != nil, "finalize rejected")
    try expect(uploadDestinationAllowed("https://uploadimg.vercel.app/api/uploads/%66inalize", method: "POST") == nil, "encoded finalize alias accepted")
}
test("production notifications describe explicit public upload and never fictitious success") {
    var request: UNNotificationRequest?
    let bridge = NativeBridge(boardFactory: { FakeBoard() }, sendNotification: { request = $0 }, removeNotifications: { _ in })
    bridge.notificationsAllowed = true
    let candidate = String(repeating: "a", count: 32)
    bridge.sync(ids: ["candidate." + candidate], revision: 1)
    bridge.notify(id: candidate, result: false, revision: 1)
    try expect(request?.content.body.contains("enlace público temporal") == true, "consent copy lacks public upload")
    let result = String(repeating: "b", count: 48)
    bridge.sync(ids: ["result." + result], revision: 2)
    bridge.notify(id: result, result: true, revision: 2)
    try expect(request?.content.body == "https://uploadimg.vercel.app/i/" + result, "notification has wrong public origin")
}
test("live-service opt-in uses a valid generated two-pixel fixture without clipboard") {
    let bytes = Data(base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEElEQVR4nGMw7rgLRAwQCgAqBgZhF+QOcgAAAABJRU5ErkJggg==")!
    let fixture = try ImageDecoder.prepare(bytes)
    try expect(fixture.image.width == 2 && fixture.image.height == 2 && fixture.image.bytes == bytes, "synthetic upload fixture is invalid or changed")
}
print("RESULT \(passed) passed, \(failed) failed")
exit(failed == 0 ? 0 : 1)
