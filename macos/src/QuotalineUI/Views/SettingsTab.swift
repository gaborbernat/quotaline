import AppKit
import QuotalineCore
import QuotalineRust
import SwiftUI

public struct SettingsTab: View {
  @Bindable var environment: UIEnvironment
  public let providerFocusRequest: ProviderSettingsFocusRequest?
  @State var confirmClear = false
  @State var confirmResetAll = false
  @State var highlightedModel: WindowKey?
  @State var focusRequest: SettingsModelFocusRequest?
  @State var labelDrafts: [WindowKey: String] = [:]
  @State var mountedSections: Set<SettingsSection>
  @State var modelsMounted: Bool
  @State var measurementHeight: CGFloat
  @State var providerRowsReady = false
  @FocusState var providerFocus: ProviderID?
  @Environment(\.dynamicTypeSize) var dynamicTypeSize
  @Environment(\.accessibilityReduceMotion) var systemReducesMotion
  let mountsIncrementally: Bool

  public init(
    environment: UIEnvironment, providerFocusRequest: ProviderSettingsFocusRequest? = nil,
    mountsIncrementally: Bool = true
  ) {
    self.environment = environment
    self.providerFocusRequest = providerFocusRequest
    self.mountsIncrementally = mountsIncrementally
    let mountedSections: Set<SettingsSection> = mountsIncrementally ? [.about] : Set(SettingsSection.allCases)
    let modelsMounted = !mountsIncrementally
    _mountedSections = State(initialValue: mountedSections)
    _modelsMounted = State(initialValue: modelsMounted)
    _measurementHeight = State(
      initialValue: PopoverGeometry.settingsHeight(
        Self.heightInput(environment: environment, mountedSections: mountedSections, modelsMounted: modelsMounted)))
  }

  var settings: QuotalineCore.Settings { environment.settings }
  var context: SettingsContextPresentation {
    SettingsContextPresentation(
      settings: settings, states: environment.state.providers, isSandboxed: environment.isSandboxed,
      loginStatus: environment.launchAtLoginStatus)
  }
  private var contentMounted: Bool {
    modelsMounted && mountedSections.count == SettingsSection.allCases.count
  }
  public var body: some View {
    ScrollViewReader { scroll in
      ScrollingTab(
        tab: .settings,
        measurementHeight: measurementHeight
      ) {
        VStack(alignment: .leading, spacing: 12) {
          ForEach(SettingsSection.allCases, id: \.self) { scope in
            section(scope) { sectionContent(scope, scroll: scroll) }
              .task {
                if scope == .providers {
                  providerRowsReady = true
                  focusProvider(providerFocusRequest, scroll: scroll)
                }
              }
          }
          HStack {
            Spacer()
            resetDefaultsButton
          }
          .padding(.top, 2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .controlSize(dynamicTypeSize.isAccessibilitySize ? .regular : .small)
      }
      .onChange(of: heightInput) { _, input in measurementHeight = PopoverGeometry.settingsHeight(input) }
      .task(id: providerFocusRequest?.id) {
        if mountsIncrementally { await mountDeferredContent() }
        guard !Task.isCancelled else { return }
        focusProvider(providerFocusRequest, scroll: scroll)
      }
    }
    .alert("Reset all settings?", isPresented: $confirmResetAll) {
      resetAlertActions
    } message: {
      Text(
        "This restores all stored settings, including model selection, short labels, provider setup, and hidden series."
      )
    }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier(contentMounted ? "settings-content-ready" : "settings-content-loading")
  }

  var resetAlertActions: some View {
    Group {
      Button("Cancel", role: .cancel, action: cancelResetAction)
        .accessibilityHint("Keeps all current settings")
      Button("Reset All Settings", role: .destructive, action: resetAllSettingsAction)
        .accessibilityHint("Restores defaults in all six sections")
    }
  }

  var cancelResetAction: () -> Void {
    { cancelResetDefaults() }
  }

  var resetAllSettingsAction: () -> Void {
    { resetDefaults() }
  }

  var heightInput: SettingsHeightInput {
    Self.heightInput(environment: environment, mountedSections: mountedSections, modelsMounted: modelsMounted)
  }

  private static func heightInput(
    environment: UIEnvironment, mountedSections: Set<SettingsSection>, modelsMounted: Bool
  ) -> SettingsHeightInput {
    let snapshots = environment.state.snapshots
    let modelCount =
      modelsMounted
      ? snapshots.values.reduce(into: 0) { count, snapshot in
        count += snapshot.windows.count { !environment.settings.hideUnusedModels || $0.usedPercent > 0 }
      } : 0
    let providerCount =
      mountedSections.contains(.providers)
      ? ProviderSettingsVisibility.providers(
        states: environment.state.providers,
        showAll: environment.settings.showAllProviders, revealed: environment.providerFocusRequest?.provider
      ).count
      : Set(snapshots.keys).count
    return SettingsHeightInput(
      mountedSections: mountedSections,
      showsModelFilter: modelsMounted,
      providerCount: providerCount,
      modelCount: modelCount,
      logLineCount: mountedSections.contains(.log) ? min(environment.log.snapshot.count, 200) : 0,
      showsCustomTemplate: environment.settings.statusFormat == .custom,
      showsUpdates: environment.canCheckForUpdates)
  }

  var reduceMotion: Bool { ReducedMotion.forced || systemReducesMotion }

  static func scrollAnimation(reduceMotion: Bool) -> Animation? {
    reduceMotion ? nil : .easeOut(duration: 0.12)
  }

  func focusProvider(_ request: ProviderSettingsFocusRequest?, scroll: ScrollViewProxy) {
    guard let request else { return }
    guard !mountsIncrementally || mountedSections.contains(.providers) else { return }
    let provider = request.provider
    withAnimation(Self.scrollAnimation(reduceMotion: reduceMotion)) {
      if let provider, providerRowsReady {
        scroll.scrollTo(SettingsFocusTarget.provider(provider), anchor: .center)
      } else {
        scroll.scrollTo(SettingsSection.providers, anchor: .center)
      }
    }
    guard providerRowsReady else { return }
    providerFocus = provider
    if environment.providerFocusRequest?.id == request.id { environment.providerFocusRequest = nil }
  }

  func section<Content: View>(_ scope: SettingsSection, @ViewBuilder content: () -> Content) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .center, spacing: 10) {
        SectionLabel(scope.title)
      }
      .accessibilityIdentifier("settings-section-\(scope.rawValue)")
      content()
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .padding(.top, 2)
    .padding(.bottom, 4)
    .overlay(alignment: .bottom) { Divider() }
  }

  func sectionContent(_ scope: SettingsSection, scroll: ScrollViewProxy) -> AnyView {
    if scope != .about, !mountedSections.contains(scope) {
      AnyView(sectionPlaceholder(scope))
    } else {
      switch scope {
      case .about: AnyView(about)
      case .menuBar: AnyView(menuBar(scroll))
      case .providers: AnyView(providers)
      case .data: AnyView(data)
      case .notifications: AnyView(notifications)
      case .log: AnyView(LogSection(environment: environment))
      }
    }
  }

  private func sectionPlaceholder(_ scope: SettingsSection) -> some View {
    Color.clear
      .frame(height: placeholderHeight(scope))
      .accessibilityHidden(true)
  }

  // About always mounts first, so it never needs a placeholder.
  private static let placeholderHeights: [SettingsSection: CGFloat] = [
    .menuBar: 620, .providers: 420, .data: 120, .notifications: 120, .log: 260,
  ]

  private func placeholderHeight(_ scope: SettingsSection) -> CGFloat {
    Self.placeholderHeights[scope]!
  }

  private func mountDeferredContent() async {
    await Task.yield()
    guard !Task.isCancelled else { return }
    mountedSections = Set(SettingsSection.allCases)
    modelsMounted = true
  }

  public func setting<Value>(_ keyPath: ReferenceWritableKeyPath<QuotalineCore.Settings, Value>) -> Binding<Value> {
    Binding(get: { settings[keyPath: keyPath] }, set: { settings[keyPath: keyPath] = $0 })
  }

  public func menuBarSetting<Value>(
    _ keyPath: ReferenceWritableKeyPath<QuotalineCore.Settings, Value>
  ) -> Binding<Value> {
    Binding(get: { settings[keyPath: keyPath] }, set: { update(keyPath, to: $0) })
  }

  func update<Value>(_ keyPath: ReferenceWritableKeyPath<QuotalineCore.Settings, Value>, to value: Value) {
    settings[keyPath: keyPath] = value
    environment.actions.settingsChanged()
  }

  public func resetDefaults() {
    settings.resetToDefaults()
    labelDrafts.removeAll()
    environment.actions.settingsReset()
    environment.actions.settingsChanged()
  }

  func requestResetDefaults() {
    confirmResetAll = true
  }

  func cancelResetDefaults() {
    confirmResetAll = false
  }

  func requestClearHistory() {
    confirmClear = true
  }

  func clearHistory() {
    environment.actions.clearHistory()
  }

  func setLaunchAtLogin(_ enabled: Bool) {
    environment.actions.setLaunchAtLogin(enabled)
  }

  public func setProvider(_ provider: ProviderID, enabled: Bool) {
    settings.setProvider(provider, enabled: enabled)
    environment.actions.settingsChanged()
  }

  public func setThreshold(_ threshold: Int, on: Bool) {
    let thresholds = QuotalineRust.notificationSetThreshold(
      thresholds: settings.notifications.thresholds.map { Int32(clamping: $0) },
      target: Int32(clamping: threshold), enabled: on)
    settings.notifications = NotificationSettings(
      enabled: settings.notifications.enabled, thresholds: thresholds.map(Int.init),
      notifyOnReset: settings.notifications.notifyOnReset,
      notifyOnAuthProblems: settings.notifications.notifyOnAuthProblems,
      notifyOnExpiringCredits: settings.notifications.notifyOnExpiringCredits,
      notifyOnPace: settings.notifications.notifyOnPace,
      playSound: settings.notifications.playSound)
  }

  public func threshold(_ threshold: Int) -> Binding<Bool> {
    Binding(get: { settings.notifications.thresholds.contains(threshold) }, set: { setThreshold(threshold, on: $0) })
  }

  public func refreshMinutes(_ provider: ProviderID) -> Binding<Int> {
    Binding(
      get: { settings.refreshInterval(for: provider) / 60 },
      set: { settings.setRefreshInterval($0 * 60, for: provider) })
  }

  public var historyRetentionDays: Binding<Int> {
    Binding(
      get: { settings.historyRetentionDays },
      set: {
        settings.historyRetentionDays = $0
        environment.actions.historyRetentionChanged()
      })
  }

  public func missingAccess(_ provider: ProviderID) -> [SandboxResource] {
    settings.missingAccess(for: provider)
  }

  public func openRepository() {
    environment.actions.openURL(environment.appInfo.repository)
  }

  public func openPrivacyPolicy() {
    environment.actions.openURL(AppInfo.privacyPolicyURL)
  }

  public func openNotices() {
    environment.actions.openURL(AppInfo.noticesURL)
  }

  public func openSetupGuide(_ provider: ProviderID) {
    environment.actions.openURL(AppInfo.setupGuideURL(for: provider))
  }

  public func grantAccess(_ resource: SandboxResource) {
    environment.actions.grantAccess(resource)
  }

  func resourceGrantAction(_ resource: SandboxResource) -> () -> Void {
    { grantAccess(resource) }
  }

  public func credentialText(_ provider: ProviderID) -> String {
    let state = environment.state.state(for: provider)
    switch state.credentialHealth {
    case .unchecked:
      return credentialLocation(provider) ?? ProviderCredentialHealth.uncheckedLabel
    case .missing:
      return ProviderCredentialHealth.missingLabel
    case .valid(let source, let expiresAt):
      return uniqueCredentialParts(
        [source.title, source.detail]
          + [expiresAt.map { "expires \($0.formatted(date: .abbreviated, time: .shortened))" }])
    case .expired(let source, let date):
      return uniqueCredentialParts([
        source.title, source.detail,
        "expired \(date.formatted(date: .abbreviated, time: .omitted))",
      ])
    case .unreadable(let source, let detail):
      return uniqueCredentialParts(
        [source?.title, source?.detail, source == nil ? credentialLocation(provider) : nil, "unreadable: \(detail)"])
    }
  }

  func credentialLocation(_ provider: ProviderID) -> String? {
    guard
      let location = environment.credentialDescriptions[provider]?.trimmingCharacters(in: .whitespacesAndNewlines),
      !location.isEmpty
    else { return nil }
    return location
  }

  private func uniqueCredentialParts(_ values: [String?]) -> String {
    var seen: Set<String> = []
    return values.compactMap { value in
      guard let value, seen.insert(value).inserted else { return nil }
      return value
    }
    .joined(separator: " · ")
  }

  public func authenticationHint(_ provider: ProviderID) -> String? {
    guard settings.isProviderActive(provider, state: environment.state.providers[provider]) else { return nil }
    let state = environment.state.state(for: provider)
    if let issue = state.recoveryIssue { return issue.detail }
    return state.availability == .authenticationRequired ? provider.loginHint : nil
  }

  func recoveryIssue(_ provider: ProviderID) -> ProviderRecoveryIssue? {
    let state = environment.state.state(for: provider)
    if let issue = state.recoveryIssue { return issue }
    if let issue = ProviderSetupState.from(
      provider: provider, enabled: true, credential: state.credentialHealth,
      resources: resourceStates(provider)
    ).issue {
      return issue
    }
    return state.availability == .authenticationRequired ? provider.setup.missingCredentialIssue : nil
  }

  func resourceStates(_ provider: ProviderID) -> [ResourceAccessState] {
    let current = environment.state.state(for: provider).resourceAccess
    if !current.isEmpty { return current }
    return provider.sandboxResources.map { resource in
      ResourceAccessState(resource: resource, health: settings.bookmark(for: resource) == nil ? .needed : .granted)
    }
  }

  func perform(_ action: ProviderRecoveryAction, provider: ProviderID, detail: String) {
    switch action {
    case .copyCommand(let command): environment.actions.copy(command)
    case .checkAgain: environment.actions.refreshProvider(provider)
    case .refreshProvider(let provider): environment.actions.refreshProvider(provider)
    case .grantAccess(let resource): grantAccess(resource)
    case .openLoginItems: environment.actions.openLoginItems()
    case .contactAdministrator: environment.actions.copy(detail)
    }
  }

  public func provider(_ provider: ProviderID) -> Binding<Bool> {
    Binding(get: { isProviderEnabled(provider) }, set: { setProvider(provider, enabled: $0) })
  }

  func isProviderEnabled(_ provider: ProviderID) -> Bool {
    settings.providerOverride(for: provider)
      ?? settings.isProviderActive(provider, state: environment.state.providers[provider])
  }

  var selection: [WindowKey] {
    settings.hasCustomSelection
      ? settings.selectedWindows : StatusItemBuilder.defaultSelection(environment.state.snapshots)
  }

  var previewModel: StatusItemModel {
    let activeProviders = settings.activeProviders(states: environment.state.providers)
    let snapshots = environment.state.snapshots.filter { activeProviders.contains($0.key) }
    let available = snapshots.keys.sorted().flatMap { provider in
      snapshots[provider]!.windows.map { (WindowKey(provider, $0), $0) }
    }
    let order = SettingsOrderDraft(
      providers: settings.providerOrder, models: settings.modelOrder, available: available.map(\.0))
    let windows = Dictionary(uniqueKeysWithValues: available)
    let labels = ShortLabelPolicy.validOverrides(
      windows: windows, persisted: settings.shortLabels, drafts: labelDrafts)
    return StatusItemBuilder.build(
      StatusItemInput(
        snapshots: snapshots,
        availability: environment.state.availability.filter { activeProviders.contains($0.key) },
        selectedKeys: order.orderedSelection(selection), format: settings.statusFormat,
        customTemplate: settings.customTemplate, decimals: settings.percentDecimals,
        hideZeroCells: settings.hideZeroCells, order: settings.windowOrder,
        labels: labels, now: environment.now, display: settings.usageDisplay))
  }

}

enum SettingsFocusTarget: Hashable {
  case providers
  case provider(ProviderID)
}
