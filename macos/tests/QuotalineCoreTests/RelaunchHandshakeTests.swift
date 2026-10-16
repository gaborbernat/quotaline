import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func relaunchHandshakeNamesTheReplacedProcessToExactlyOneReader() {
  let defaults = testDefaults()
  RelaunchHandshake.request(from: 4242, in: defaults)

  #expect(RelaunchHandshake.take(from: defaults) == 4242)
  #expect(RelaunchHandshake.take(from: defaults) == nil)
}

@Test func relaunchHandshakeIgnoresAWithdrawnRequest() {
  let defaults = testDefaults()
  RelaunchHandshake.request(from: 4242, in: defaults)
  RelaunchHandshake.withdraw(in: defaults)

  #expect(RelaunchHandshake.take(from: defaults) == nil)
}

@Test @MainActor func settingsRequestRelaunchesThroughTheirOwnDefaults() {
  let defaults = testDefaults()
  let settings = Settings(defaults: defaults)

  settings.requestRelaunch(from: 7)
  #expect(RelaunchHandshake.take(from: defaults) == 7)

  settings.requestRelaunch(from: 7)
  settings.withdrawRelaunch()
  #expect(RelaunchHandshake.take(from: defaults) == nil)
}
