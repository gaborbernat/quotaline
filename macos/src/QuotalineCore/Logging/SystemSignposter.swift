import OSLog

public struct DiagnosticSignposter: Sendable {
  private let base: OSSignposter

  public init(category: LogCategory) {
    base = OSSignposter(subsystem: SystemLog.subsystem, category: category.rawValue)
  }

  public func withInterval<Result>(
    _ name: StaticString, operation: () throws -> Result
  ) rethrows -> Result {
    guard base.isEnabled else { return try operation() }
    let state = base.beginInterval(name)
    defer { base.endInterval(name, state) }
    return try operation()
  }

  public func withInterval<Result: Sendable>(
    _ name: StaticString, operation: () async throws -> Result
  ) async rethrows -> Result {
    guard base.isEnabled else { return try await operation() }
    let state = base.beginInterval(name)
    defer { base.endInterval(name, state) }
    return try await operation()
  }
}

public enum DiagnosticSignposts {
  public static let geometry = DiagnosticSignposter(category: .geometry)
  public static let refresh = DiagnosticSignposter(category: .refresh)
  public static let tabs = DiagnosticSignposter(category: .tabs)
}
