import AppKit

/// Render the existing artwork inside a padded macOS app icon.
func render() throws {
  guard CommandLine.arguments.count == 3,
    let image = NSImage(contentsOfFile: CommandLine.arguments[1]),
    let bitmap = NSBitmapImageRep(
      bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024, bitsPerSample: 8,
      samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
      bytesPerRow: 0, bitsPerPixel: 0),
    let context = NSGraphicsContext(bitmapImageRep: bitmap)
  else { throw CocoaError(.fileReadCorruptFile) }

  // match the visual size and corner treatment of macOS application icons
  NSGraphicsContext.saveGraphicsState()
  NSGraphicsContext.current = context
  let bounds = NSRect(x: 100, y: 100, width: 824, height: 824)
  NSBezierPath(roundedRect: bounds, xRadius: 185, yRadius: 185).addClip()
  image.draw(in: bounds)
  NSGraphicsContext.restoreGraphicsState()
  guard let png = bitmap.representation(using: .png, properties: [:]) else {
    throw CocoaError(.fileWriteUnknown)
  }
  try png.write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
}

try render()
