import QuotalineCore
import SwiftUI

public struct CloudCreditsView: View {
  @Environment(\.usageValueAppearance) private var valueAppearance
  public let presentation: UsageCloudPresentation

  public init(presentation: UsageCloudPresentation) {
    self.presentation = presentation
  }

  public var body: some View {
    VStack(alignment: .leading, spacing: 5) {
      Text(presentation.title).font(.callout.weight(.medium))
      Text(presentation.summary)
        .font(.callout.monospacedDigit())
        .semanticForeground(valueAppearance.foreground)
        .fixedSize(horizontal: false, vertical: true)
        .accessibilityValue(valueAppearance.accessibilityValue(presentation.summary))
      UsageBar(
        percent: presentation.usedPercent,
        color: presentation.muted ? Color(.tertiary) : Color(UsageColor.color(percent: presentation.usedPercent)),
        label: presentation.title)
    }
  }
}
