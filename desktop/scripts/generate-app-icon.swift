import AppKit
import Foundation

// Editable 1024-point artwork derived from docs/assets/brand.svg.
// Preserve its image glyph, rounded strokes and blue tile; use transparent
// macOS margins, a restrained blue gradient and a soft shadow for the app icon.
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)

func blue(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat) -> NSColor {
    NSColor(srgbRed: red / 255, green: green / 255, blue: blue / 255, alpha: 1)
}

func drawArtwork() {
    let tile = NSBezierPath(roundedRect: NSRect(x: 80, y: 80, width: 864, height: 864),
                            xRadius: 194, yRadius: 194)
    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor(srgbRed: 0.04, green: 0.15, blue: 0.38, alpha: 0.28)
    shadow.shadowBlurRadius = 28
    shadow.shadowOffset = NSSize(width: 0, height: -12)
    shadow.set()
    blue(37, 99, 235).setFill()
    tile.fill()
    NSGraphicsContext.restoreGraphicsState()
    NSGradient(starting: blue(68, 136, 255), ending: blue(24, 74, 207))!
        .draw(in: tile, angle: 90)

    // An inset edge catches light without adding another symbol to the brand.
    NSColor.white.withAlphaComponent(0.22).setStroke()
    let edge = NSBezierPath(roundedRect: NSRect(x: 82, y: 82, width: 860, height: 860),
                            xRadius: 192, yRadius: 192)
    edge.lineWidth = 3
    edge.stroke()

    NSGraphicsContext.saveGraphicsState()
    let glyphTransform = NSAffineTransform()
    glyphTransform.translateX(by: 192, yBy: 192)
    glyphTransform.scale(by: 640 / 24)
    glyphTransform.concat()
    let outline = NSBezierPath(roundedRect: NSRect(x: 3, y: 3, width: 18, height: 18),
                               xRadius: 2, yRadius: 2)
    let landscape = NSBezierPath()
    landscape.move(to: NSPoint(x: 3, y: 16))
    for point in [NSPoint(x: 8, y: 11), NSPoint(x: 13, y: 16),
                  NSPoint(x: 16, y: 13), NSPoint(x: 21, y: 18)] {
        landscape.line(to: point)
    }
    let dot = NSBezierPath()
    dot.move(to: NSPoint(x: 15, y: 7))
    dot.line(to: NSPoint(x: 15.01, y: 7))
    NSColor.white.setStroke()
    for path in [outline, landscape, dot] {
        path.lineWidth = 1.75
        path.lineCapStyle = .round
        path.lineJoinStyle = .round
        path.stroke()
    }
    NSGraphicsContext.restoreGraphicsState()
}

func render(size: Int, filename: String) throws {
    guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size,
        pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
        isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
        let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
        fatalError("Cannot create icon bitmap")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.imageInterpolation = .high
    let transform = NSAffineTransform()
    transform.translateX(by: 0, yBy: CGFloat(size))
    transform.scaleX(by: CGFloat(size) / 1024, yBy: -CGFloat(size) / 1024)
    transform.concat()
    drawArtwork()
    context.flushGraphics()
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else {
        fatalError("Cannot encode icon PNG")
    }
    try png.write(to: output.appendingPathComponent(filename), options: .atomic)
}

for size in [16, 32, 128, 256, 512] {
    try render(size: size, filename: "icon_\(size)x\(size).png")
    try render(size: size * 2, filename: "icon_\(size)x\(size)@2x.png")
}
