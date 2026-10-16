import Darwin
import QuotalineRust

public struct LibprocProcessScanner: ProcessScanner {
  private let scanner = QuotalineRust.AntigravityNativeScanner()

  public init() {}

  public func discoverServers() -> [LocalLanguageServer] {
    scanner.discoverServers().map { LocalLanguageServer(pid: $0.pid, port: $0.port, csrfToken: $0.csrfToken) }
  }
}
