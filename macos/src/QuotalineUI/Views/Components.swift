import AppKit
import QuotalineCore
import SwiftUI

extension Color {
  public init(_ hsb: HSBColor) {
    self.init(hue: hsb.hue, saturation: hsb.saturation, brightness: hsb.brightness)
  }
}

public struct UsageBar: View {
  public let percent: Double
  public let expectedPercent: Double?
  public let color: Color
  public let height: CGFloat
  public let label: String
  public let outlined: Bool

  public init(
    percent: Double, expectedPercent: Double? = nil, color: Color, height: CGFloat = 5, label: String,
    outlined: Bool = false
  ) {
    self.percent = percent
    self.expectedPercent = expectedPercent
    self.color = color
    self.height = height
    self.label = label
    self.outlined = outlined
  }

  public var body: some View {
    GeometryReader { proxy in
      ZStack(alignment: .leading) {
        Capsule().fill(Color.primary.opacity(0.06))
        let fraction = min(max(percent, 0), 100) / 100
        if fraction > 0 {
          Capsule()
            .fill(color.gradient)
            .frame(width: max(proxy.size.width * fraction, height))
        }
        if let expectedPercent {
          let expected = min(max(expectedPercent, 0), 100) / 100
          RoundedRectangle(cornerRadius: 1)
            .fill(Color.primary)
            .frame(width: 2, height: height + 4)
            .offset(x: max(min(proxy.size.width * expected - 1, proxy.size.width - 2), 0))
        }
        if outlined {
          Capsule().strokeBorder(Color.primary, lineWidth: 1)
        }
      }
    }
    .frame(height: height)
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(label)
    .accessibilityValue(
      expectedPercent.map { "\(Format.percent(percent)) used, expected \(Format.percent($0))" }
        ?? "\(Format.percent(percent)) used")
  }
}

public struct Banner: View {
  public enum Tone {
    case warning
    case info
  }

  public let text: String
  public let tone: Tone

  public init(_ text: String, tone: Tone = .warning) {
    self.text = text
    self.tone = tone
  }

  public var body: some View {
    HStack(alignment: .top, spacing: 6) {
      Image(systemName: tone == .warning ? "exclamationmark.triangle.fill" : "info.circle.fill")
        .semanticForeground(tone == .warning ? .warning : .secondary)
        .accessibilityHidden(true)
      LinkifiedText(text)
        .font(.body)
        .textSelection(.enabled)
      Spacer(minLength: 0)
    }
    .padding(8)
    .background(
      Color(tone == .warning ? .warning : .secondary).opacity(0.12), in: RoundedRectangle(cornerRadius: 8)
    )
    .accessibilityElement(children: .combine)
    // The tint is the only thing that separates a warning from a note, so the label says which one this is.
    .accessibilityLabel(Text(verbatim: "\(tone == .warning ? "Warning" : "Note"): \(text)"))
  }
}

public struct LinkifiedText: View {
  public let text: String

  public init(_ text: String) {
    self.text = text
  }

  public var body: some View {
    Text(Self.attributed(text))
  }

  public static func attributed(_ text: String) -> AttributedString {
    var result = AttributedString(text)
    // Compiling a Regex costs more than scanning the string, and every banner does this on every body evaluation.
    for span in links(in: text) {
      if let range = Range(span, in: result), let url = URL(string: String(text[span])) {
        result[range].link = url
        result[range].underlineStyle = .single
      }
    }
    return result
  }

  /// The `http://` and `https://` runs in `text`, each ending at the first space or closing bracket.
  static func links(in text: String) -> [Range<String.Index>] {
    var spans: [Range<String.Index>] = []
    var cursor = text.startIndex
    while let scheme = text.range(of: "http", range: cursor..<text.endIndex) {
      cursor = scheme.upperBound
      let rest = text[scheme.upperBound...]
      guard rest.hasPrefix("://") || rest.hasPrefix("s://") else { continue }
      let end = text[scheme.lowerBound...].firstIndex { $0.isWhitespace || $0 == ")" } ?? text.endIndex
      spans.append(scheme.lowerBound..<end)
      cursor = end
    }
    return spans
  }
}

public struct NativeActionButton<Label: View>: View {
  public let intent: ControlIntent
  public let action: () -> Void
  public let label: Label

  public init(
    intent: ControlIntent = .action, action: @escaping () -> Void, @ViewBuilder label: () -> Label
  ) {
    self.intent = intent
    self.action = action
    self.label = label()
  }

  public var body: some View {
    Button(role: role, action: action) { label }
      .buttonStyle(.bordered)
      .semanticControl(intent)
  }

  var role: ButtonRole? {
    switch intent {
    case .destructive: .destructive
    default: nil
    }
  }
}

/// On the macOS 27 SDK, AppKit draws push and rounded bezels at a fixed height, so a wrapped title or a two-line image
/// overflows the bezel and its edge strokes through the content. This cell paints the fill SwiftUI's bordered buttons
/// use at whatever height the content needs.
class FilledButtonCell: NSButtonCell {
  override func drawBezel(withFrame frame: NSRect, in controlView: NSView) {
    (isHighlighted || state == .on ? NSColor.systemFill : .secondarySystemFill).setFill()
    NSBezierPath(roundedRect: frame, xRadius: 5, yRadius: 5).fill()
  }
}

final class NativeButtonContainer: NSView {
  let button: NSButton

  init(button: NSButton) {
    self.button = button
    super.init(frame: .zero)
    button.frame = bounds
    button.autoresizingMask = [.width, .height]
    addSubview(button)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) is unavailable")
  }
}

public struct NativeSegmentedControl<Value: Hashable>: NSViewRepresentable {
  @Binding private var selection: Value
  private let values: [Value]
  private let labels: [String]
  private let accessibilityLabel: String
  private let accessibilityIdentifier: String?

  public init(
    _ items: [(value: Value, label: String)],
    selection: Binding<Value>,
    accessibilityLabel: String,
    accessibilityIdentifier: String? = nil
  ) {
    values = items.map(\.value)
    labels = items.map(\.label)
    _selection = selection
    self.accessibilityLabel = accessibilityLabel
    self.accessibilityIdentifier = accessibilityIdentifier
  }

  public func makeNSView(context: Context) -> NSSegmentedControl {
    let control = NSSegmentedControl(
      labels: labels,
      trackingMode: .selectOne,
      target: context.coordinator,
      action: #selector(Coordinator.changed(_:)))
    control.controlSize = .small
    control.segmentStyle = .automatic
    let font = NSFont.systemFont(ofSize: NSFont.smallSystemFontSize)
    control.font = font
    for (index, label) in labels.enumerated() {
      control.setWidth(
        ceil(NSAttributedString(string: label, attributes: [.font: font]).size().width) + 24, forSegment: index)
    }
    control.setContentCompressionResistancePriority(.required, for: .horizontal)
    control.setAccessibilityLabel(accessibilityLabel)
    if let accessibilityIdentifier { control.setAccessibilityIdentifier(accessibilityIdentifier) }
    return control
  }

  public func updateNSView(_ control: NSSegmentedControl, context: Context) {
    let selectedSegment = values.firstIndex(of: selection) ?? -1
    if control.selectedSegment != selectedSegment { control.selectedSegment = selectedSegment }
    context.coordinator.selection = $selection
    context.coordinator.values = values
  }

  public func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSSegmentedControl, context: Context) -> CGSize? {
    nsView.fittingSize
  }

  public func makeCoordinator() -> Coordinator {
    Coordinator(selection: $selection, values: values)
  }

  @MainActor
  public final class Coordinator: NSObject {
    var selection: Binding<Value>
    var values: [Value]

    init(selection: Binding<Value>, values: [Value]) {
      self.selection = selection
      self.values = values
    }

    @objc func changed(_ sender: NSSegmentedControl) {
      guard values.indices.contains(sender.selectedSegment) else { return }
      selection.wrappedValue = values[sender.selectedSegment]
    }
  }
}

extension NativeActionButton where Label == Text {
  public init(_ title: String, intent: ControlIntent = .action, action: @escaping () -> Void) {
    self.init(intent: intent, action: action) { Text(title) }
  }
}

public struct NativeIconButton: View {
  public let symbol: String
  public let accessibilityLabel: String
  public let explanation: String
  public let intent: ControlIntent
  public let action: () -> Void

  public init(
    symbol: String,
    accessibilityLabel: String,
    explanation: String? = nil,
    intent: ControlIntent = .action,
    action: @escaping () -> Void
  ) {
    self.symbol = symbol
    self.accessibilityLabel = accessibilityLabel
    self.explanation = explanation ?? accessibilityLabel
    self.intent = intent
    self.action = action
  }

  public init(symbol: String, help: String, intent: ControlIntent = .action, action: @escaping () -> Void) {
    self.init(symbol: symbol, accessibilityLabel: help, explanation: help, intent: intent, action: action)
  }

  public var body: some View {
    NativeActionButton(intent: intent, action: action) {
      Label(accessibilityLabel, systemImage: symbol).labelStyle(.iconOnly)
    }
    .buttonBorderShape(.circle)
    .richHelp(TooltipContent(title: accessibilityLabel, body: explanation))
    .accessibilityLabel(accessibilityLabel)
  }
}

public typealias IconButton = NativeIconButton
