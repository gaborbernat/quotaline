import Foundation
import QuotalineRust

public enum UsagePresenter {
  public static func iconTone(_ state: [ProviderID: ProviderState]) -> StatusIconTone {
    let availability = state.values.map(\.availability)
    if availability.contains(.authenticationRequired) { return .attention }
    if availability.contains(.networkUnavailable) { return .offline }
    return .normal
  }

  /// Groups neighbouring rows that share a reset instant, preserving the order they were selected in.
  public static func groups(_ rows: [WindowRow]) -> [WindowRowGroup] {
    let edges = QuotalineRust.resetGroupEdges(
      windows: rows.map {
        ResetGroupInput(
          resetAt: $0.window.resetsAt?.timeIntervalSince1970,
          dayPrecision: $0.window.resetPrecision == .day)
      })
    var result: [WindowRowGroup] = []
    for (row, edge) in zip(rows, edges) {
      let reset = row.window.resetsAt
      if edge.withPrevious {
        let last = result.removeLast()
        result.append(WindowRowGroup(rows: last.rows + [row], resetText: last.resetText))
      } else {
        let text = reset == nil ? nil : "Resets in \(row.countdown) · \(row.resetClock)"
        result.append(WindowRowGroup(rows: [row], resetText: text))
      }
    }
    return result
  }

  public static func cards(
    state: [ProviderID: ProviderState], enabled: Set<ProviderID>, samples: [WindowKey: [UsageSample]],
    options: UsageDisplayOptions = .standard, now: Date
  ) -> [ProviderCard] {
    state.keys.sorted().filter { isVisible(state[$0]!, enabled: enabled.contains($0)) }.map { provider in
      card(provider: provider, state: state[provider]!, samples: samples, options: options, now: now)
    }
  }

  public static func presentation(
    state: [ProviderID: ProviderState], enabled: Set<ProviderID>, selected: Set<WindowKey>,
    samples: [WindowKey: [UsageSample]], analytics: [ProviderID: UsageAnalyticsPresentation], lastRefresh: Date?,
    iconTone: StatusIconTone, isRefreshing: Bool, options: UsageDisplayOptions = .standard, now: Date
  ) -> UsagePresentation {
    let cards = state.keys.sorted().filter { isVisible(state[$0]!, enabled: enabled.contains($0)) }.map { provider in
      card(
        provider: provider, state: state[provider]!, samples: samples, selected: selected,
        analytics: analytics[provider] ?? UsageAnalyticsPresentation(), options: options, now: now)
    }
    return UsagePresentation(
      builtAt: now, lastRefresh: lastRefresh, iconTone: iconTone, isRefreshing: isRefreshing, cards: cards,
      emptyTitle: enabled.isEmpty ? SharedCopy.noProvidersEnabled : SharedCopy.noUsageAvailable,
      emptyDescription: SharedCopy.setUpProvidersHint)
  }

  public static func isVisible(_ state: ProviderState, enabled: Bool) -> Bool {
    QuotalineRust.usageCardVisible(
      enabled: enabled, snapshot: state.snapshot != nil, analytics: state.analytics != nil,
      credentialUsable: state.credentialHealth.isUsable)
  }

  public static func card(
    provider: ProviderID, state: ProviderState, samples: [WindowKey: [UsageSample]],
    options: UsageDisplayOptions = .standard, now: Date
  ) -> ProviderCard {
    card(
      provider: provider, state: state, samples: samples, selected: nil,
      analytics: analyticsPresentation(state.analytics, now: now), options: options, now: now)
  }

  static func card(
    provider: ProviderID, state: ProviderState, samples: [WindowKey: [UsageSample]], selected: Set<WindowKey>?,
    analytics: UsageAnalyticsPresentation, options: UsageDisplayOptions, now: Date
  ) -> ProviderCard {
    let snapshot = state.snapshot
    let rows = (snapshot?.windows ?? []).filter { window in
      selected?.contains(WindowKey(provider, window)) ?? true
    }.map { window -> WindowRow in
      let key = WindowKey(provider, window)
      let pace = PaceEstimate.estimate(
        window: window, samples: samples[key] ?? [], workdays: options.paceWorkdays, now: now)
      let paceText = pace.summary(now: now)
      let copy = QuotaCopy(window: window, pace: pace, paceText: paceText)
      return WindowRow(
        key: key,
        window: window,
        pace: pace,
        countdown: Format.countdown(to: window.resetsAt, now: now),
        resetClock: Format.resetClock(window.resetsAt, precision: window.resetPrecision, now: now),
        detail: copy.detail,
        paceLabel: pace.status.title,
        paceText: paceText,
        helpText: copy.help,
        isSelected: selected?.contains(key) ?? true,
        display: options.display
      )
    }
    let (title, description) = emptyState(provider: provider, state: state)
    let spend = snapshot?.spend.map { spendPresentation($0, provider: provider, now: now) }
    let cloud = snapshot?.cloudCredits.map { cloudPresentation($0, stale: state.availability == .stale, now: now) }
    let credits = creditsPresentation(snapshot?.credits, resetCredits: snapshot?.resetCredits, provider: provider)
    let local = snapshot?.localUsage.map(localPresentation)
    let identity = snapshot?.identity
    let visibleLimits = Set(rows.filter { $0.window.usedPercent >= 100 }.map(\.window.id))
    let notices = snapshot?.notices ?? []
    let visibleNoticeIndices = Set(
      QuotalineRust.visibleNoticeIndices(
        candidates: notices.map {
          NoticeCandidate(limitReached: $0.kind == .limitReached, windowId: $0.windowID)
        },
        fullWindows: visibleLimits.sorted()
      ).map(Int.init))
    return ProviderCard(
      provider: provider,
      availability: state.availability,
      identity: options.hidePersonalInformation ? identity?.hidingPersonalInformation : identity,
      chips: chips(snapshot: snapshot, hidePersonalInformation: options.hidePersonalInformation),
      rows: rows,
      credits: snapshot?.credits,
      spend: snapshot?.spend,
      resetCredits: snapshot?.resetCredits,
      notices: notices.enumerated().filter { visibleNoticeIndices.contains($0.offset) }.map { _, notice in
        guard notice.kind == .limitReached, let id = notice.windowID,
          let window = snapshot?.window(id), window.resetPrecision == .instant, let reset = window.resetsAt
        else { return notice }
        return Notice(
          kind: notice.kind, text: "\(window.label) limit reached.", windowID: id, resetsAt: reset)
      },
      warnings: state.warnings,
      lastError: state.lastError,
      presentedAt: now,
      fetchedAt: snapshot?.fetchedAt,
      fetchedAge: Format.relativeAge(snapshot?.fetchedAt, now: now),
      source: snapshot?.source,
      emptyTitle: title,
      emptyDescription: description,
      isRefreshing: state.isRefreshing,
      localUsage: snapshot?.localUsage,
      codeReviews: analytics.codeReviews,
      groups: groups(rows),
      spendPresentation: spend,
      cloudPresentation: cloud,
      creditsPresentation: credits,
      localPresentation: local,
      details: (snapshot?.details ?? []).filter {
        // Older caches include a row listing fields the provider did not report.
        $0.id != "oauth-availability" && !$0.value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      }
    )
  }

  public static func analyticsPresentation(_ analytics: ProviderAnalytics?, now: Date) -> UsageAnalyticsPresentation {
    UsageAnalyticsPresentation(codeReviews: codeReviewSummary(analytics, now: now))
  }

  static func codeReviewSummary(_ analytics: ProviderAnalytics?, now: Date) -> String? {
    guard let analytics else { return nil }
    guard
      let counts = QuotalineRust.reviewCounts(
        rows: analytics.points.filter { $0.metric == .codeReviews }.map {
          ReviewRowRecord(day: $0.day, value: $0.value)
        },
        now: now.timeIntervalSince1970)
    else { return nil }
    return "\(counts.today) today · \(counts.week) this week"
  }

  public static func spendPresentation(
    _ spend: SpendControl, provider: ProviderID, now: Date
  ) -> UsageSpendPresentation {
    let projection = spendProjection(spend)
    let metrics = projection.metrics.map { metric in
      let (title, help) = spendMetricCopy(metric.kind)
      let value: String
      switch metric.kind {
      case .remaining: value = CreditAllowance.formatted(spend.creditAllowance!.remaining!)
      case .allowanceSource: value = Format.humanize(spend.creditAllowance!.source!)
      case .balance: value = spend.balance!.formatted
      case .resets: value = Format.resetClock(spend.resetsAt!, now: now)
      case .autoReload: value = spend.autoReload! ? "On" : "Off"
      case .purchase: value = "Available"
      }
      return UsageMetricPresentation(title: title, value: value, help: help)
    }
    return UsageSpendPresentation(
      spend: spend, provider: provider, title: provider == .claude ? SharedCopy.usageCredits : SharedCopy.spendControl,
      summary: spendSummary(spend, projection: projection), metrics: metrics)
  }

  private static func spendMetricCopy(_ kind: SpendMetricKind) -> (String, String) {
    switch kind {
    case .remaining: ("Remaining", "Provider-reported monthly credits, not currency.")
    case .allowanceSource: ("Allowance source", "Who sets this credit allowance.")
    case .balance: ("Balance", "Prepaid credit balance.")
    case .resets: ("Resets", "When the monthly spend counter resets.")
    case .autoReload: ("Auto-reload", "Whether credits top up automatically.")
    case .purchase:
      ("Purchase", "The provider reports that this account can purchase credits. This app does not make purchases.")
    }
  }

  public static func cloudPresentation(_ credits: CloudCredits, stale: Bool, now: Date) -> UsageCloudPresentation {
    let projection = QuotalineRust.usageCloudCreditProjection(
      input: CloudCreditFactsInput(
        limit: credits.limit, remaining: credits.remaining,
        expiresAt: credits.expiresAt.map { Int64($0.timeIntervalSince1970) },
        locked: credits.lockedReason != nil),
      now: Int64(now.timeIntervalSince1970))
    let money = { (value: Double) in
      value.formatted(.currency(code: "USD").precision(.fractionLength(2)))
    }
    var summary = "\(money(credits.remaining)) of \(money(credits.limit)) left"
    switch projection.status {
    case .locked: summary += " · Unavailable"
    case .expired: summary += " · Expired"
    case .active:
      if let expiry = projection.expiryEpoch {
        let expiryDate = Date(timeIntervalSince1970: Double(expiry))
        summary += " · Expires \(expiryDate.formatted(date: .numeric, time: .standard))"
      }
    }
    return UsageCloudPresentation(
      title: SharedCopy.cloudSessionCredits, summary: summary, usedPercent: projection.usedPercent,
      muted: stale || projection.status != .active)
  }

  public static func creditsPresentation(
    _ credits: CreditBalance?, resetCredits: ResetCredits?, provider: ProviderID = .codex
  ) -> UsageCreditsPresentation? {
    let metrics = QuotalineRust.usageCreditMetrics(
      credits: credits.map { value in
        CreditUsageInput(
          unlimited: value.unlimited, overageLimitReached: value.overageLimitReached,
          localMessages: value.approxLocalMessages.map {
            CreditRangeInput(lower: Int64($0.lowerBound), upper: Int64($0.upperBound))
          },
          cloudMessages: value.approxCloudMessages.map {
            CreditRangeInput(lower: Int64($0.lowerBound), upper: Int64($0.upperBound))
          })
      },
      resets: resetCredits.map { value in
        CreditResetInput(
          available: UInt64(value.available), applicable: UInt64(value.applicable),
          totalEarned: value.totalEarned.map(UInt64.init), purchaseEligible: value.immediatePurchaseEligible)
      },
      expiries: resetCredits?.expiries.map { Int64($0.expiresAt.timeIntervalSince1970) } ?? []
    ).map { metric in
      let (title, value, help) = creditMetricCopy(metric, credits: credits, resets: resetCredits, provider: provider)
      return UsageMetricPresentation(
        title: title, value: value, help: help, isSupportingDetail: metric.supporting)
    }
    return metrics.isEmpty
      ? nil : UsageCreditsPresentation(credits: credits, resetCredits: resetCredits, metrics: metrics)
  }

  private static func creditMetricCopy(
    _ metric: CreditMetricRecord, credits: CreditBalance?, resets: ResetCredits?, provider: ProviderID
  ) -> (String, String, String) {
    switch metric.kind {
    case .credits:
      let value =
        credits!.unlimited ? "Unlimited" : credits!.balance == nil ? "Balance not reported" : credits!.formattedBalance
      let help =
        provider == .codex
        ? "Available usage credits shared by ChatGPT Work and Codex. Credits extend usage beyond plan limits; they are separate from limit-reset credits."
        : "Credits extend usage beyond plan limits."
      return ("Credits", value, help)
    case .overage: return ("Overage", SharedCopy.limitReached, "The credit overage cap has been hit.")
    case .localMessages:
      let range = credits!.approxLocalMessages!
      return (
        "Local messages", "~\(range.lowerBound)–\(range.upperBound)",
        "Estimated local task messages the balance covers."
      )
    case .cloudMessages:
      let range = credits!.approxCloudMessages!
      return (
        "Cloud messages", "~\(range.lowerBound)–\(range.upperBound)",
        "Estimated cloud task messages the balance covers."
      )
    case .limitResets:
      return (
        "Limit resets", "\(resets!.available) available",
        "Usage-limit resets available to redeem; \(resets!.applicable) apply now."
      )
    case .nextExpiry:
      let value = Date(timeIntervalSince1970: TimeInterval(metric.epoch!))
        .formatted(date: .abbreviated, time: .shortened)
      return (
        "Next reset-credit expiry", value,
        "The earliest reported expiry among available reset credits. Expired credits cannot reset a limit."
      )
    case .resetPurchase:
      return (
        "Reset purchase", "Eligible",
        "The provider reports that an immediate reset can be purchased. This app does not make purchases."
      )
    case .resetsEarned: return ("Resets earned", String(resets!.totalEarned!), "Reset credits earned so far.")
    }
  }

  public static func localPresentation(_ usage: LocalUsage) -> UsageLocalPresentation {
    UsageLocalPresentation(
      usage: usage,
      metrics: QuotalineRust.usageLocalMetrics(
        windowCost: usage.windowCost, costPerHour: usage.costPerHour, todayCost: usage.todayCost
      ).map { metric in
        let (title, help) = localMetricCopy(metric.kind)
        let value: String
        switch metric.kind {
        case .windowTokens: value = Format.compactNumber(Double(usage.windowTokens)) + " tokens"
        case .windowCost: value = localMoney(usage.windowCost, digits: metric.moneyFractionDigits!)
        case .costPerHour: value = localMoney(usage.costPerHour, digits: metric.moneyFractionDigits!) + "/hr"
        case .todayTokens: value = Format.compactNumber(Double(usage.todayTokens)) + " tokens"
        case .todayMessages: value = String(usage.todayMessages)
        case .todayCost: value = localMoney(usage.todayCost, digits: metric.moneyFractionDigits!)
        }
        return UsageMetricPresentation(title: title, value: value, help: help)
      })
  }

  private static func localMetricCopy(_ kind: LocalMetricKind) -> (String, String) {
    switch kind {
    case .windowTokens:
      (SharedCopy.fiveHourBlock, SharedCopy.sessionTokensHelp)
    case .windowCost:
      (SharedCopy.blockCost, SharedCopy.apiEquivalentCostHelp)
    case .costPerHour: (SharedCopy.burnRate, SharedCopy.burnRateHelp)
    case .todayTokens: ("Today", SharedCopy.tokensTodayHelp)
    case .todayMessages: (SharedCopy.messagesToday, SharedCopy.messagesTodayHelp)
    case .todayCost: (SharedCopy.todayCost, "API-equivalent cost of today's traffic.")
    }
  }

  public static func localMoney(_ value: Double) -> String {
    localMoney(value, digits: QuotalineRust.usageLocalMoneyFractionDigits(value: value))
  }

  private static func localMoney(_ value: Double, digits: UInt8) -> String {
    value.formatted(.currency(code: "USD").precision(.fractionLength(Int(digits))))
  }

  static func chips(snapshot: ProviderReading?, hidePersonalInformation: Bool = false) -> [Chip] {
    guard let snapshot else { return [] }
    let identity = snapshot.identity
    let view = QuotalineRust.usageIdentityPresentation(
      email: identity?.email, organization: identity?.organization, hide: hidePersonalInformation)
    var chips: [Chip] = []
    if let plan = identity?.planName { chips.append(Chip(text: plan)) }
    if let email = view.email { chips.append(Chip(text: email)) }
    if view.showOrganization, let organization = view.organization { chips.append(Chip(text: organization)) }
    if let renewal = identity?.subscriptionActiveUntil {
      chips.append(Chip(text: "Renews \(renewal.formatted(date: .abbreviated, time: .omitted))"))
    }
    if snapshot.source == .localLog { chips.append(Chip(text: "From local logs")) }
    return chips
  }

  static func emptyState(provider: ProviderID, state: ProviderState) -> (String, String) {
    let name = provider.displayName
    switch state.availability {
    case .loading: return ("Loading \(name)", "Fetching usage from \(name)…")
    case .authenticationRequired: return (SharedCopy.noUsageAvailable, "Review \(name) under Settings > Providers.")
    case .networkUnavailable: return ("\(name) is offline", state.lastError ?? "No network connection.")
    case .disabled: return ("\(name) disabled", "Enable it under Settings > Providers.")
    case .unavailable: return ("\(name) unavailable", state.lastError ?? "The usage endpoint returned an error.")
    case .rateLimited: return ("\(name) rate limited", state.lastError ?? "The usage endpoint asked us to slow down.")
    case .current, .stale: return (SharedCopy.noUsageYet, "\(name) reports no active limits.")
    }
  }

  public static func spendSummary(_ spend: SpendControl) -> String {
    spendSummary(spend, projection: spendProjection(spend))
  }

  private static func spendProjection(_ spend: SpendControl) -> SpendProjectionRecord {
    QuotalineRust.usageSpendProjection(
      input: SpendFactsInput(
        usedCredits: spend.creditAllowance?.used != nil, limitCredits: spend.creditAllowance?.limit != nil,
        usedMoney: spend.used != nil, limitMoney: spend.limit != nil, percent: spend.percent,
        remainingCredits: spend.creditAllowance?.remaining != nil,
        allowanceSource: spend.creditAllowance?.source != nil,
        balance: spend.balance != nil, reset: spend.resetsAt != nil, autoReload: spend.autoReload != nil,
        purchase: spend.canPurchaseCredits))
  }

  private static func spendSummary(_ spend: SpendControl, projection: SpendProjectionRecord) -> String {
    var parts: [String] = []
    if let status = spendStatus(spend) { parts.append(status) }
    if let used = projection.used, let limit = projection.limit {
      parts.append("\(spendAmount(used, spend: spend, used: true)) of \(spendAmount(limit, spend: spend, used: false))")
    }
    if let percent = projection.percent { parts.append("\(Format.percent(percent)) used") }
    return parts.isEmpty ? SharedCopy.usageUnavailable : parts.joined(separator: " · ")
  }

  private static func spendStatus(_ spend: SpendControl) -> String? {
    switch spend.enabled {
    case .some(false): spend.disabledReason.map { "Off (\(Format.humanize($0)))" } ?? "Off"
    case .none: "Enablement not reported"
    case .some(true): nil
    }
  }

  private static func spendAmount(_ source: SpendAmount, spend: SpendControl, used: Bool) -> String {
    switch source {
    case .credits:
      CreditAllowance.formatted(used ? spend.creditAllowance!.used! : spend.creditAllowance!.limit!)
    case .money: (used ? spend.used! : spend.limit!).formatted
    case .unavailable: "Unavailable"
    case .unreportedLimit: SharedCopy.unreportedLimit
    }
  }

}
