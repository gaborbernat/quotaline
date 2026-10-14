import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func historyPresenterHidesAndIsolatesWindows() async throws {
  let (presenter, settings, _) = try await loadedHistoryPresenterFixture()
  let key = WindowKey(provider: .claude, windowID: "session")
  let codex = WindowKey(provider: .codex, windowID: "session")
  presenter.toggleVisibility(key)
  await presenter.waitForLoad()
  #expect(!presenter.isVisible(key))
  #expect(presenter.request(now: fixedNow).keys == [codex])
  presenter.toggleVisibility(codex)
  await presenter.waitForLoad()
  #expect(presenter.isVisible(codex))
  presenter.toggleVisibility(key)
  await presenter.waitForLoad()
  #expect(presenter.isVisible(key))
  presenter.isolate(key)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys == [codex])
  presenter.isolate(key)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys.isEmpty)
}

@Test @MainActor func historyPresenterIsolationRestoresPreviouslyHiddenSeries() async throws {
  let (presenter, settings, _) = try await loadedHistoryPresenterFixture()
  let hidden = WindowKey(provider: .gemini, windowID: "retired")
  let claude = WindowKey(provider: .claude, windowID: "session")
  let codex = WindowKey(provider: .codex, windowID: "session")
  settings.historyHiddenKeys = [hidden]
  presenter.redraw()
  await presenter.waitForLoad()

  presenter.isolate(claude)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys == [hidden, codex])
  presenter.isolate(claude)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys == [hidden])
}

@Test @MainActor func historyPresenterIgnoresUnknownIsolation() async throws {
  let (presenter, settings, _) = try await loadedHistoryPresenterFixture()
  let claude = WindowKey(provider: .claude, windowID: "session")
  let codex = WindowKey(provider: .codex, windowID: "session")

  presenter.isolate(claude)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys == [codex])

  presenter.isolate(WindowKey(provider: .gemini, windowID: "retired"))
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys == [codex])

  presenter.isolate(claude)
  await presenter.waitForLoad()
  #expect(settings.historyHiddenKeys.isEmpty)
}
