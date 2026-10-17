import AppKit
import QuotalineCore
import SwiftUI

/// Renders the popover tabs straight to PNG for the website. A screen capture of the live popover picks up its shadow
/// and whatever sits behind it, so the export hosts the same view offscreen and keeps the content alone.
@MainActor
public enum PopoverExporter {
  /// Renders the native view offscreen at the requested size.
  public static func image(_ view: some View, dark: Bool, size: CGSize? = nil) -> NSImage? {
    image(NSHostingView(rootView: view), dark: dark, size: size)
  }

  public static func image(_ hosting: NSView, dark: Bool, size: CGSize? = nil) -> NSImage? {
    let appearance = NSAppearance(named: dark ? .darkAqua : .aqua)!
    hosting.appearance = appearance
    // The popover paints on a translucent material with no window behind it here, so back it with the colour the
    // website draws behind the shot. windowBackgroundColor would resolve against whatever appearance the exporting
    // process happens to run under, which is how light shots ended up grey.
    hosting.wantsLayer = true
    hosting.layer?.backgroundColor = Brand.card(dark: dark).cgColor
    hosting.frame = CGRect(origin: .zero, size: size ?? hosting.fittingSize)
    hosting.layoutSubtreeIfNeeded()
    guard hosting.bounds.width > 0, hosting.bounds.height > 0,
      let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: Int(ceil(hosting.bounds.width * 2)), pixelsHigh: Int(ceil(hosting.bounds.height * 2)),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
        bytesPerRow: 0, bitsPerPixel: 0)
    else { return nil }
    rep.size = hosting.bounds.size
    appearance.performAsCurrentDrawingAppearance { hosting.cacheDisplay(in: hosting.bounds, to: rep) }
    let image = NSImage(size: hosting.bounds.size)
    image.addRepresentation(rep)
    return image
  }

  public static func png(_ view: some View, dark: Bool, size: CGSize? = nil) -> Data? {
    png(NSHostingView(rootView: view), dark: dark, size: size)
  }

  public static func png(_ hosting: NSView, dark: Bool, size: CGSize? = nil) -> Data? {
    guard let image = image(hosting, dark: dark, size: size), let rep = image.representations.first as? NSBitmapImageRep
    else {
      return nil
    }
    return rep.representation(using: .png, properties: [:])
  }
}

@MainActor
final class MenuTarget: NSObject {
  private weak var controller: AppController?

  init(controller: AppController) {
    self.controller = controller
  }

  func menu() -> NSMenu {
    controller?.contextMenu() ?? NSMenu()
  }

  @objc func run(_ sender: NSMenuItem) {
    guard let id = sender.representedObject as? String else { return }
    controller?.run(id)
  }
}

@MainActor
public final class LogWindowController: NSWindowController {
  public let log: LogBuffer
  private let present: @MainActor (NSWindow, Any?) -> Void

  public convenience init(log: LogBuffer, presentsWindow: Bool = true) {
    self.init(log: log, present: LiveDependencies.windowPresentation(enabled: presentsWindow))
  }

  init(log: LogBuffer, present: @escaping @MainActor (NSWindow, Any?) -> Void) {
    self.log = log
    self.present = present
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 720, height: 480), styleMask: [.titled, .closable, .resizable],
      backing: .buffered, defer: false)
    window.title = "Quotaline Log"
    window.contentView = NSHostingView(rootView: FullLogView(log: log))
    window.center()
    super.init(window: window)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  public override func showWindow(_ sender: Any?) {
    present(window!, sender)
  }
}
