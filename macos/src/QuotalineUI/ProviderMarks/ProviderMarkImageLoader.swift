import AppKit
import QuotalineCore

@MainActor
public final class ProviderMarkImageLoader {
  public static let shared = ProviderMarkImageLoader()

  private struct Key: Hashable {
    let provider: ProviderID
    let appearance: ProviderMarkAppearance
  }

  private var images: [Key: NSImage] = [:]
  private var unavailable: Set<Key> = []
  private let resourceURL: (String) -> URL?

  init(resourceURL: @escaping (String) -> URL? = ProviderMarkCatalog.resourceURL(named:)) {
    self.resourceURL = resourceURL
  }

  public func image(for provider: ProviderID, appearance: ProviderMarkAppearance) -> NSImage? {
    let key = Key(provider: provider, appearance: appearance)
    if let image = images[key] { return image }
    if unavailable.contains(key) { return nil }
    let descriptor = ProviderMarkCatalog.descriptor(for: provider, appearance: appearance)
    guard
      let url = resourceURL(descriptor.resourceName),
      let image = NSImage(contentsOf: url)
    else {
      unavailable.insert(key)
      return nil
    }
    // The icons declare a 1em size, which would rasterise at one point before scaling up.
    image.size = NSSize(width: 24, height: 24)
    image.isTemplate = !descriptor.keepsOriginalColors
    image.cacheMode = .always
    images[key] = image
    return image
  }
}
