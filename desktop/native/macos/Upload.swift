import Foundation

// No persistent cache, cookies, credentials, redirects or HTTP error details.
// Rust calls this only on its dedicated upload worker, never the main thread.
final class UploadResponse: NSObject, URLSessionDataDelegate {
    let finished = DispatchSemaphore(value: 0)
    private let lock = NSLock()
    private var body = Data()
    private var status: Int32 = 0
    private var failed = false
    func urlSession(_ session: URLSession, task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest,
        completionHandler: @escaping (URLRequest?) -> Void) {
        lock.lock(); failed = true; lock.unlock()
        completionHandler(nil)
    }
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask,
        didReceive response: URLResponse, completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        guard let http = response as? HTTPURLResponse, response.expectedContentLength <= 65_536 else {
            lock.lock(); failed = true; lock.unlock(); completionHandler(.cancel); return
        }
        lock.lock(); status = Int32(http.statusCode); lock.unlock()
        completionHandler(.allow)
    }
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.lock()
        if body.count + data.count > 65_536 { failed = true; lock.unlock(); dataTask.cancel(); return }
        body.append(data); lock.unlock()
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        lock.lock(); failed = failed || error != nil; lock.unlock(); finished.signal()
    }
    func result() -> (Int32, Data) {
        lock.lock(); defer { lock.unlock() }
        return failed ? (0, Data()) : (status, body)
    }
}
public typealias UploadCallback = @convention(c) (Int32, UnsafePointer<UInt8>?, Int, UnsafeMutableRawPointer?) -> Void

func uploadDestinationAllowed(_ address: String, method: String) -> URL? {
    guard let url = URL(string: address), let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
        url.scheme == "https", url.user == nil, url.password == nil,
        url.port == nil, url.fragment == nil,
        ((url.host == "uploadimg.vercel.app" && url.query == nil && method == "POST"
          && ["/api/uploads/prepare", "/api/uploads/token", "/api/uploads/finalize"].contains(components.percentEncodedPath))
         || (url.host == "vercel.com" && components.percentEncodedPath == "/api/blob/" && url.query != nil && method == "PUT"))
    else { return nil }
    return url
}

func performUpload(_ request: URLRequest, timeout: Double, configuration: URLSessionConfiguration) -> (Int32, Data) {
    let delegate = UploadResponse()
    let session = URLSession(configuration: configuration, delegate: delegate, delegateQueue: nil)
    let task = session.dataTask(with: request); task.resume()
    guard delegate.finished.wait(timeout: .now() + timeout / 1000) == .success else {
        task.cancel(); session.invalidateAndCancel(); return (0, Data())
    }
    let result = delegate.result()
    session.finishTasksAndInvalidate()
    return result
}

@_cdecl("uig_http_request")
public func uploadRequest(_ metadata: UnsafePointer<UInt8>?, _ metadataLength: Int,
    _ bytes: UnsafePointer<UInt8>?, _ length: Int,
    _ callback: UploadCallback, _ context: UnsafeMutableRawPointer?) {
    guard !Thread.isMainThread, let metadata, metadataLength > 0, metadataLength <= 32_768,
        let bytes, length > 0, length <= 10_000_000,
        let object = try? JSONSerialization.jsonObject(with: Data(bytes: metadata, count: metadataLength)) as? [String: Any],
        let address = object["url"] as? String, address.count <= 32_768,
        let method = object["method"] as? String, let timeout = object["timeoutMs"] as? Double,
        timeout >= 1, timeout <= 90_000,
        let headers = object["headers"] as? [String: String], headers.count <= 8,
        let url = uploadDestinationAllowed(address, method: method)
    else { callback(0, nil, 0, context); return }
    let config = URLSessionConfiguration.ephemeral
    config.urlCache = nil; config.httpCookieStorage = nil; config.urlCredentialStorage = nil
    config.httpShouldSetCookies = false; config.connectionProxyDictionary = [:]
    config.timeoutIntervalForRequest = timeout / 1000
    config.timeoutIntervalForResource = timeout / 1000
    var request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: timeout / 1000)
    request.httpMethod = method; request.httpBody = Data(bytes: bytes, count: length)
    for (key, value) in headers { request.setValue(value, forHTTPHeaderField: key) }
    let (status, response) = performUpload(request, timeout: timeout, configuration: config)
    response.withUnsafeBytes { buffer in
        callback(status, buffer.bindMemory(to: UInt8.self).baseAddress, response.count, context)
    }
}
