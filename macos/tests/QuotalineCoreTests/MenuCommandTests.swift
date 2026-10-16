import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func menuHoldsOnlyRefreshAndQuit() {
  let commands = MenuCommand.menu(canCheckForUpdates: false, appName: "Quotaline")
  #expect(commands == [.refresh, .separator, .quit(appName: "Quotaline")])
  #expect(commands.first?.title == "Refresh Now")
  #expect(commands.first?.keyEquivalent == "r")
  #expect(commands.last?.title == "Quit Quotaline")
  #expect(commands.last?.keyEquivalent == "q")
}

@Test func menuAddsTheUpdateCommandOnlyWhenTheBuildCanCheck() {
  let without = MenuCommand.menu(canCheckForUpdates: false, appName: "App")
  #expect(!without.contains(.checkForUpdates))
  let with = MenuCommand.menu(canCheckForUpdates: true, appName: "App")
  #expect(with.contains(.checkForUpdates))
  #expect(with.count == without.count + 2)
}

@Test func menuCommandIdentifiersAreStable() {
  #expect(MenuCommand.refresh.id == "refresh")
  #expect(MenuCommand.separator.id == "separator")
  #expect(MenuCommand.separator.title.isEmpty)
  #expect(MenuCommand.separator.keyEquivalent.isEmpty)
  #expect(MenuCommand.checkForUpdates.id == "updates")
  #expect(MenuCommand.checkForUpdates.title == "Check for Updates…")
  #expect(MenuCommand.quit(appName: "A").id == MenuCommand.quit(appName: "B").id)
}
