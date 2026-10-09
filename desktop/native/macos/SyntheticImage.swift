import Foundation
import CoreGraphics
import ImageIO

// All callers keep these synthetic fixtures in memory except the isolated file tests.
enum SyntheticImage {
    static func make(width: Int = 64, height: Int = 64, type: String = "public.png", frames: Int = 1,
                     srgb: Bool = false) throws -> Data {
        let space = srgb ? CGColorSpace(name: CGColorSpace.sRGB)! : CGColorSpaceCreateDeviceRGB()
        guard let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                                      bytesPerRow: width * 4, space: space,
                                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
            throw ProbeError.invalidImage
        }
        let color = srgb ? CGColor(colorSpace: space, components: [0.2, 0.5, 0.8, 1])! :
            CGColor(red: 0.2, green: 0.5, blue: 0.8, alpha: 1)
        context.setFillColor(color)
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        guard let image = context.makeImage() else { throw ProbeError.invalidImage }
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, type as CFString, frames, nil) else {
            throw ProbeError.invalidImage
        }
        for _ in 0..<frames { CGImageDestinationAddImage(destination, image, nil) }
        guard CGImageDestinationFinalize(destination) else { throw ProbeError.invalidImage }
        return data as Data
    }
}
