import AppKit
import Darwin

// Offscreen controls only: this harness creates no window/status item, starts no
// native bridge and changes no application/system appearance or permissions.
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let icon = menuBarIcon()
guard icon.isTemplate, icon.size == NSSize(width: 18, height: 18) else {
    fatalError("Expected an 18-point native template")
}

func render(_ image: NSImage, size: Int) -> NSBitmapImageRep {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    let context = NSGraphicsContext(bitmapImageRep: bitmap)!
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = context
    image.draw(in: NSRect(x: 0, y: 0, width: size, height: size), from: .zero,
        operation: .sourceOver, fraction: 1)
    NSGraphicsContext.restoreGraphicsState()
    return bitmap
}

let glyph = render(icon, size: 180)
var ink = 0; var transparent = 0
for y in 0..<180 {
    for x in 0..<180 {
        let alpha = glyph.colorAt(x: x, y: y)!.alphaComponent
        if alpha > 0.1 { ink += 1 }
        if alpha == 0 { transparent += 1 }
        if x < 10 || y < 10 || x >= 170 || y >= 170 {
            precondition(alpha == 0, "Glyph clipped at the transparent inset")
        }
    }
}
precondition(ink > 2_000 && ink < 12_000 && transparent > 20_000,
    "Expected a sparse outlined glyph with transparent background")
try glyph.representation(using: .png, properties: [:])!.write(to: output.appendingPathComponent("glyph.png"))
print("PASS vector raster: 180px, ink \(ink), transparent \(transparent), empty edge inset")

var brightness: [String: Double] = [:]
for (name, appearance) in [("light", NSAppearance.Name.aqua), ("dark", NSAppearance.Name.darkAqua)] {
    let button = NSButton(frame: NSRect(x: 0, y: 0, width: 24, height: 24))
    button.isBordered = false; button.imagePosition = .imageOnly
    button.title = ""; button.image = icon; button.appearance = NSAppearance(named: appearance)
    let bitmap = button.bitmapImageRepForCachingDisplay(in: button.bounds)!
    button.appearance!.performAsCurrentDrawingAppearance {
        button.cacheDisplay(in: button.bounds, to: bitmap)
    }
    var sum = 0.0; var pixels = 0
    for y in 0..<bitmap.pixelsHigh {
        for x in 0..<bitmap.pixelsWide {
            let color = bitmap.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
            if color.alphaComponent > 0.1 {
                sum += Double((color.redComponent + color.greenComponent + color.blueComponent) / 3)
                pixels += 1
            }
        }
    }
    precondition(pixels > 40, "Offscreen control did not render the image")
    brightness[name] = sum / Double(pixels)
    try bitmap.representation(using: .png, properties: [:])!.write(to: output.appendingPathComponent("\(name)-control.png"))
    print("RENDER offscreen NSButton \(name) appearance: \(bitmap.pixelsWide) × \(bitmap.pixelsHigh), ink brightness \(brightness[name]!)")
}
precondition(brightness["light"]! < 0.4 && brightness["dark"]! > 0.6,
    "Template did not adapt to the two control appearances")
print("PASS AppKit template control renders dark ink in Aqua and light ink in Dark Aqua")
print("No live menu-bar/hover/dark-mode observation is claimed by this harness.")
