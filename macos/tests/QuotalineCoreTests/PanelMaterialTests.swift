import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test(arguments: PlatformDesignGeneration.allCases, PanelSurfaceRole.allCases)
func panelMaterialPolicyCoversEveryGenerationAndSurface(
  generation: PlatformDesignGeneration,
  surface: PanelSurfaceRole
) {
  let expected: PanelMaterial =
    switch surface {
    case .popoverChrome: .system
    case .content: .standardContent
    }
  #expect(PanelMaterialPolicy.material(for: surface, generation: generation) == expected)
}

@Test func platformDesignGenerationsMatchSupportedPolicyForks() {
  #expect(PlatformDesignGeneration.allCases == [.macOS14, .macOS15, .macOS26])
}
