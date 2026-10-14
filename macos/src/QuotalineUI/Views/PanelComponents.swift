import QuotalineCore
import SwiftUI

public struct SectionLabel: View {
  public let title: String

  public init(_ title: String) {
    self.title = title
  }

  public var body: some View {
    Text(title)
      .textCase(.uppercase)
      .font(.caption2.weight(.semibold))
      .tracking(0.6)
      .semanticForeground(.secondary)
      .accessibilityAddTraits(.isHeader)
  }
}

public struct PanelSection<Content: View>: View {
  public let title: String
  public let content: Content

  public init(_ title: String, @ViewBuilder content: () -> Content) {
    self.title = title
    self.content = content()
  }

  public var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      SectionLabel(title)
      content
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

public struct ResponsivePanelLayout<Wide: View, Narrow: View>: View {
  public let wide: Wide
  public let narrow: Narrow
  @Environment(\.dynamicTypeSize) private var dynamicTypeSize

  public init(@ViewBuilder wide: () -> Wide, @ViewBuilder narrow: () -> Narrow) {
    self.wide = wide()
    self.narrow = narrow()
  }

  public var body: some View {
    if dynamicTypeSize.isAccessibilitySize {
      narrow
    } else {
      ViewThatFits(in: .horizontal) {
        wide
        narrow
      }
    }
  }
}

public struct PanelRow<Content: View>: View {
  public let title: String
  public let labelWidth: CGFloat
  public let content: Content

  public init(_ title: String, labelWidth: CGFloat = 116, @ViewBuilder content: () -> Content) {
    self.title = title
    self.labelWidth = labelWidth
    self.content = content()
  }

  public var body: some View {
    ResponsivePanelLayout {
      HStack(alignment: .firstTextBaseline, spacing: 12) {
        label.frame(width: labelWidth, alignment: .leading)
        content.frame(maxWidth: .infinity, alignment: .leading)
      }
    } narrow: {
      VStack(alignment: .leading, spacing: 5) {
        if !title.isEmpty { label }
        content.frame(maxWidth: .infinity, alignment: .leading)
      }
    }
  }

  private var label: some View {
    Text(title).semanticForeground(.secondary)
  }
}

public struct ChipView: View {
  public let chip: Chip
  public let onCopy: (String) -> Void

  public init(chip: Chip, onCopy: @escaping (String) -> Void) {
    self.chip = chip
    self.onCopy = onCopy
  }

  public var body: some View {
    ControlGroup {
      Button(chip.text, action: primaryAction)
        .accessibilityHint("Copies this value")
      Button(action: copyAction) { Label("Copy \(chip.text)", systemImage: "doc.on.doc").labelStyle(.iconOnly) }
        .richHelp(TooltipContent(title: "Copy", body: "Copies this value to the clipboard."))
        .accessibilityLabel("Copy \(chip.text)")
    }
    .buttonStyle(.bordered)
    .controlSize(.small)
    .semanticControl(.action)
    .contextMenu {
      Button("Copy", systemImage: "doc.on.doc", action: copyAction)
    }
  }

  public func primaryAction() {
    onCopy(chip.text)
  }

  public func copyAction() {
    onCopy(chip.text)
  }
}

public struct WrappingHStack: Layout {
  public let horizontalSpacing: CGFloat
  public let verticalSpacing: CGFloat

  public init(horizontalSpacing: CGFloat = 6, verticalSpacing: CGFloat = 6) {
    self.horizontalSpacing = horizontalSpacing
    self.verticalSpacing = verticalSpacing
  }

  public func makeCache(subviews: Subviews) -> Cache {
    Cache()
  }

  public func updateCache(_ cache: inout Cache, subviews: Subviews) {
    cache = Cache()
  }

  public func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) -> CGSize {
    let rows = layoutRows(width: proposal.width ?? .infinity, subviews: subviews, cache: &cache)
    let height = rows.map { $0.height }.reduce(0, +) + CGFloat(max(rows.count - 1, 0)) * verticalSpacing
    let width = rows.map { $0.width }.max() ?? 0
    return CGSize(width: proposal.width ?? width, height: height)
  }

  public func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) {
    var rowTop = bounds.minY
    for row in layoutRows(width: bounds.width, subviews: subviews, cache: &cache) {
      var itemLeft = bounds.minX
      for item in row.items {
        subviews[item.index].place(
          at: CGPoint(x: itemLeft, y: rowTop + (row.height - item.size.height) / 2),
          proposal: ProposedViewSize(item.size))
        itemLeft += item.size.width + horizontalSpacing
      }
      rowTop += row.height + verticalSpacing
    }
  }

  public struct Cache {
    var width: CGFloat?
    var rows: [Row] = []
  }

  struct Row {
    var items: [(index: Int, size: CGSize)] = []
    var width: CGFloat = 0
    var height: CGFloat = 0
  }

  func layoutRows(width: CGFloat, subviews: Subviews, cache: inout Cache) -> [Row] {
    if cache.width == width { return cache.rows }
    var rows: [Row] = []
    var current = Row()
    for (index, subview) in subviews.enumerated() {
      let size = subview.sizeThatFits(.unspecified)
      let spacing = current.items.isEmpty ? 0 : horizontalSpacing
      if !current.items.isEmpty, current.width + spacing + size.width > width {
        rows.append(current)
        current = Row()
      }
      current.items.append((index, size))
      current.width += (current.items.count == 1 ? 0 : horizontalSpacing) + size.width
      current.height = max(current.height, size.height)
    }
    if !current.items.isEmpty { rows.append(current) }
    cache.width = width
    cache.rows = rows
    return rows
  }
}

public struct SizeKey: PreferenceKey {
  public static let defaultValue: CGSize = .zero

  /// Keeping the last non-zero value would make a second publisher anywhere in the subtree win by reduction order,
  /// silently sizing the popover to some inner view.
  public static func reduce(value: inout CGSize, nextValue: () -> CGSize) {
    let next = nextValue()
    value = CGSize(width: max(value.width, next.width), height: max(value.height, next.height))
  }
}

extension View {
  /// Reports this view's size to the enclosing `onPreferenceChange(SizeKey.self)` without observing it here.
  public func publishSize() -> some View {
    background(GeometryReader { proxy in Color.clear.preference(key: SizeKey.self, value: proxy.size) })
  }

  public func measureSize(_ onChange: @escaping @MainActor @Sendable (CGSize) -> Void) -> some View {
    publishSize().onPreferenceChange(SizeKey.self) { size in Task { @MainActor in onChange(size) } }
  }
}

public struct MetricCell: View {
  @Environment(\.usageValueAppearance) private var valueAppearance
  public let title: String
  public let value: String
  public let help: String

  public init(title: String, value: String, help: String) {
    self.title = title
    self.value = value
    self.help = help
  }

  public var body: some View {
    VStack(alignment: .leading, spacing: 2) {
      Text(title).font(.callout).semanticForeground(.secondary)
      Text(value).font(.body.monospacedDigit()).fixedSize(horizontal: false, vertical: true)
        .semanticForeground(valueAppearance.foreground)
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .richHelp(TooltipContent(title: title, body: help))
    .accessibilityElement(children: .ignore)
    .accessibilityLabel("\(title): \(value)")
    .accessibilityValue(valueAppearance.accessibilityValue(value))
  }
}

public struct HelpText: View {
  public let text: String

  public init(_ text: String) {
    self.text = text
  }

  public var body: some View {
    Text(text).font(.callout).frame(idealWidth: 260, maxWidth: 260, alignment: .leading)
  }
}

public struct EmptyStateView: View {
  public let title: String
  public let systemImage: String
  public let description: String

  public init(title: String, systemImage: String, description: String) {
    self.title = title
    self.systemImage = systemImage
    self.description = description
  }

  public var body: some View {
    HStack(alignment: .top, spacing: 12) {
      Image(systemName: systemImage).font(.system(size: 26)).semanticForeground(.secondary).frame(width: 32)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 3) {
        Text(title).font(.headline)
        Text(description).font(.callout).semanticForeground(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
    }
    .padding(.vertical, 8)
    .frame(maxWidth: .infinity, alignment: .leading)
    .accessibilityElement(children: .combine)
  }
}
