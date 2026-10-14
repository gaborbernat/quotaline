import Darwin
import QuotalineRust

/// `ps` and `lsof` trigger a TCC prompt on macOS 26, so the platform adapter uses libproc.
public protocol ProcessScanner: Sendable {
  func discoverServers() -> [LocalLanguageServer]
}

public struct LocalLanguageServer: Sendable, Equatable {
  public let pid: pid_t
  public let port: UInt16
  public let csrfToken: String?

  public init(pid: pid_t, port: UInt16, csrfToken: String?) {
    self.pid = pid
    self.port = port
    self.csrfToken = csrfToken
  }
}

final class AntigravityServerScannerAdapter: AntigravityServerScanner {
  private let scanner: any ProcessScanner

  init(_ scanner: any ProcessScanner) {
    self.scanner = scanner
  }

  func discoverServers() -> [AntigravityLocalServer] {
    scanner.discoverServers().map {
      AntigravityLocalServer(pid: $0.pid, port: $0.port, csrfToken: $0.csrfToken)
    }
  }
}
