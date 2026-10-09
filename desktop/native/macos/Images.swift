import Foundation
import AppKit
import ImageIO
import CryptoKit
import Darwin

struct FrozenImage {
    let bytes: Data
    let digest: String
    let width: Int
    let height: Int
}
struct PreparedImage {
    let image: FrozenImage
    let preview: CGImage
}
enum ProbeError: Error { case invalidImage, tooLarge, animated, unsafeFile, busyDirectory }
enum ImageDecoder {
    static func decode(_ bytes: Data) throws -> FrozenImage {
        try prepare(bytes).image
    }

    // Entire decode, canonicalization and thumbnail rasterization runs on the app's
    // one worker. Only the small independent raster crosses into GUI presentation.
    static func prepare(_ bytes: Data) throws -> PreparedImage {
        guard !bytes.isEmpty, bytes.count <= 10_000_000 else { throw ProbeError.tooLarge }
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithData(bytes as CFData, options) else {
            throw ProbeError.invalidImage
        }
        guard CGImageSourceGetCount(source) == 1 else { throw ProbeError.animated }
        guard let type = CGImageSourceGetType(source) as String?,
              ["public.png", "public.jpeg", "public.tiff"].contains(type),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, options) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0, width <= 8192, height <= 8192,
              width <= 16_000_000 / height else { throw ProbeError.invalidImage }
        guard let decoded = CGImageSourceCreateImageAtIndex(source, 0,
                  [kCGImageSourceShouldCacheImmediately: true,
                   kCGImageSourceShouldAllowFloat: false] as CFDictionary),
              decoded.width == width, decoded.height == height,
              let space = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                  bytesPerRow: width * 4, space: space,
                  bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue),
              let pixels = context.data else { throw ProbeError.invalidImage }
        context.setBlendMode(.copy)
        context.draw(decoded, in: CGRect(x: 0, y: 0, width: width, height: height))
        var digest = SHA256()
        digest.update(data: Data("\(width)x\(height):".utf8))
        digest.update(data: Data(bytes: pixels, count: width * height * 4))
        let pixelHash = digest.finalize().map { String(format: "%02x", $0) }.joined()
        var transportBytes = bytes
        if type == "public.tiff" {
            let output = NSMutableData()
            guard let destination = CGImageDestinationCreateWithData(output, "public.png" as CFString, 1, nil) else {
                throw ProbeError.invalidImage
            }
            CGImageDestinationAddImage(destination, decoded, nil)
            guard CGImageDestinationFinalize(destination), output.length <= 10_000_000 else {
                throw ProbeError.tooLarge
            }
            transportBytes = output as Data
        }
        let scale = min(1, 512 / Double(max(width, height)))
        let previewWidth = max(1, Int(Double(width) * scale))
        let previewHeight = max(1, Int(Double(height) * scale))
        guard let raster = context.makeImage(),
              let thumbnail = CGContext(data: nil, width: previewWidth, height: previewHeight,
                  bitsPerComponent: 8, bytesPerRow: previewWidth * 4, space: space,
                  bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue) else {
            throw ProbeError.invalidImage
        }
        thumbnail.setBlendMode(.copy)
        thumbnail.interpolationQuality = .high
        thumbnail.draw(raster, in: CGRect(x: 0, y: 0, width: previewWidth, height: previewHeight))
        guard let preview = thumbnail.makeImage() else { throw ProbeError.invalidImage }
        let image = FrozenImage(bytes: transportBytes, digest: pixelHash, width: width, height: height)
        return PreparedImage(image: image, preview: preview)
    }
}
protocol ImagePasteboard {
    var denied: Bool { get }
    var changeCount: Int { get }
    var imageTypes: [String]? { get }
    func imageData(type: String) -> Data?
}
enum ClipboardObservation {
    case unchanged, denied, unsupported, tooLarge, image(Data)
}
final class ClipboardMonitor {
    private(set) var enabled = false
    private var baselineCount = 0
    func enable(_ board: ImagePasteboard) -> Bool {
        guard !board.denied else { enabled = false; return false }
        baselineCount = board.changeCount
        enabled = true
        return true
    }
    func disable() { enabled = false }
    func poll(_ board: ImagePasteboard) -> ClipboardObservation {
        guard enabled else { return .unchanged }
        guard !board.denied else { enabled = false; return .denied }
        let count = board.changeCount
        guard count != baselineCount else { return .unchanged }
        baselineCount = count
        let observation = explicitPaste(board)
        if case .denied = observation { enabled = false }
        return observation
    }
    func explicitPaste(_ board: ImagePasteboard) -> ClipboardObservation {
        guard !board.denied else { return .denied }
        guard let types = board.imageTypes else { return .denied }
        guard let type = ["public.png", "public.tiff", "public.jpeg"].first(where: { types.contains($0) }) else {
            return .unsupported
        }
        guard let bytes = board.imageData(type: type) else { return .denied }
        guard !bytes.isEmpty, bytes.count <= 10_000_000 else { return .tooLarge }
        return .image(bytes)
    }
}
