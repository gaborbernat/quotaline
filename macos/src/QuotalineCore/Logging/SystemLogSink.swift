import Foundation
import OSLog

enum SystemLogSink {
  static func record(_ message: String, level: LogLevel, category: LogCategory) {
    logger(category).log(level: level.osLogType, "\(message, privacy: .private(mask: .hash))")
  }

  static func record(_ event: DiagnosticEvent, level: LogLevel) {
    let logger = logger(event.category)
    switch event {
    case .panel(let value):
      let anchor = value.anchor?.description ?? "-"
      let screen = value.screenID ?? "-"
      let screenFrame = value.screenFrame?.description ?? "-"
      let result = value.resultFrame?.description ?? "-"
      let frontmost = value.frontmostBundleID ?? "-"
      logger.log(
        level: level.osLogType,
        """
        panel action=\(value.action.rawValue, privacy: .public) \
        trigger=\(value.trigger, privacy: .public) tab=\(value.tab, privacy: .public) \
        anchor=\(anchor, privacy: .public) screen=\(screen, privacy: .public) \
        screenFrame=\(screenFrame, privacy: .public) max=\(value.maximum.description, privacy: .public) \
        proposed=\(value.proposed.description, privacy: .public) \
        clamped=\(value.clamped.description, privacy: .public) result=\(result, privacy: .public) \
        appActive=\(value.appActive, privacy: .public) \
        frontmost=\(frontmost, privacy: .private(mask: .hash))
        """
      )
    case .tab(let value):
      let from = value.from ?? "-"
      let to = value.to ?? "-"
      let source = value.sourceTab ?? "-"
      let filedUnder = value.filedUnderTab ?? "-"
      let size = value.size?.description ?? "-"
      logger.log(
        level: level.osLogType,
        """
        tab action=\(value.action.rawValue, privacy: .public) from=\(from, privacy: .public) \
        to=\(to, privacy: .public) source=\(source, privacy: .public) \
        active=\(value.activeTab, privacy: .public) filedUnder=\(filedUnder, privacy: .public) \
        size=\(size, privacy: .public)
        """
      )
    case .status(let value):
      let frame = value.buttonFrame?.description ?? "-"
      let oldTier = value.oldTier ?? -1
      let newTier = value.newTier ?? -1
      let context = value.layoutContext ?? "-"
      logger.log(
        level: level.osLogType,
        """
        status action=\(value.action.rawValue, privacy: .public) \
        trigger=\(value.trigger, privacy: .public) frame=\(frame, privacy: .public) \
        oldTier=\(oldTier, privacy: .public) newTier=\(newTier, privacy: .public) \
        visible=\(value.visible, privacy: .public) popoverVisible=\(value.popoverVisible, privacy: .public) \
        context=\(context, privacy: .private(mask: .hash))
        """
      )
    case .refresh(let value):
      let skipReason = value.skipReason?.rawValue ?? "-"
      logger.log(
        level: level.osLogType,
        """
        refresh cycle=\(value.cycleID, privacy: .public) trigger=\(value.trigger, privacy: .public) \
        provider=\(value.provider.rawValue, privacy: .public) \
        usagePolicy=\(value.usagePolicy, privacy: .public) \
        analyticsPolicy=\(value.analyticsPolicy, privacy: .public) \
        outcome=\(value.outcome.rawValue, privacy: .public) skipReason=\(skipReason, privacy: .public) \
        durationMs=\(value.durationMilliseconds, privacy: .public) \
        includeAnalytics=\(value.includeAnalytics, privacy: .public) \
        analyticsReturned=\(value.analyticsReturned, privacy: .public) \
        analyticsPoints=\(value.analyticsPointCount, privacy: .public) \
        warnings=\(value.warnings.count, privacy: .public)
        """
      )
    case .request(let value):
      let status = value.status ?? 0
      let domain = value.errorDomain ?? "-"
      let code = value.errorCode ?? 0
      logger.log(
        level: level.osLogType,
        """
        request id=\(value.requestID, privacy: .public) operation=\(value.operation, privacy: .public) \
        method=\(value.method, privacy: .public) status=\(status, privacy: .public) \
        bytes=\(value.byteCount, privacy: .public) \
        durationMs=\(value.durationMilliseconds, privacy: .public) \
        errorDomain=\(domain, privacy: .public) errorCode=\(code, privacy: .public)
        """
      )
    }
  }

  static func persistenceError(_ error: any Error) {
    let value = error as NSError
    persistence.error(
      "log file operation failed domain=\(value.domain, privacy: .public) code=\(value.code, privacy: .public)")
  }

  private static func logger(_ category: LogCategory) -> Logger {
    switch category {
    case .app: app
    case .geometry: geometry
    case .network: network
    case .persistence: persistence
    case .refresh: refresh
    case .status: status
    case .tabs: tabs
    }
  }

  private static let app = Logger(subsystem: SystemLog.subsystem, category: LogCategory.app.rawValue)
  private static let geometry = Logger(subsystem: SystemLog.subsystem, category: LogCategory.geometry.rawValue)
  private static let network = Logger(subsystem: SystemLog.subsystem, category: LogCategory.network.rawValue)
  private static let persistence = Logger(subsystem: SystemLog.subsystem, category: LogCategory.persistence.rawValue)
  private static let refresh = Logger(subsystem: SystemLog.subsystem, category: LogCategory.refresh.rawValue)
  private static let status = Logger(subsystem: SystemLog.subsystem, category: LogCategory.status.rawValue)
  private static let tabs = Logger(subsystem: SystemLog.subsystem, category: LogCategory.tabs.rawValue)
}

extension LogLevel {
  fileprivate var osLogType: OSLogType {
    switch self {
    case .debug: .debug
    case .info: .info
    case .warning: .default
    case .error: .error
    }
  }
}
