import QuotalineCore

struct StubProcessScanner: ProcessScanner {
  let servers: [LocalLanguageServer]

  init(_ servers: [LocalLanguageServer] = []) {
    self.servers = servers
  }

  func discoverServers() -> [LocalLanguageServer] { servers }
}

let antigravityServer = LocalLanguageServer(pid: 4242, port: 4321, csrfToken: "csrf-1")

let antigravityCLI = LocalLanguageServer(pid: 4343, port: 5353, csrfToken: nil)
