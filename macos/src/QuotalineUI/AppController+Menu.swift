import AppKit
import QuotalineCore
import SwiftUI

extension AppController {
  public func contextMenu() -> NSMenu {
    let menu = NSMenu()
    let commands = MenuCommand.menu(
      canCheckForUpdates: dependencies.updater?.canCheck == true, appName: dependencies.appInfo.name)
    for command in commands {
      guard command != .separator else {
        menu.addItem(.separator())
        continue
      }
      let item = NSMenuItem(
        title: command.title, action: #selector(MenuTarget.run(_:)), keyEquivalent: command.keyEquivalent)
      item.representedObject = command.id
      item.target = menuTarget
      menu.addItem(item)
    }
    return menu
  }

  public func run(_ commandID: String) {
    switch commandID {
    case MenuCommand.refresh.id: refreshNow()
    case MenuCommand.checkForUpdates.id: dependencies.updater?.checkForUpdates()
    case MenuCommand.quit(appName: "").id: dependencies.terminate()
    default: return
    }
  }

  @discardableResult
  public func exportHistory() -> Task<Void, Never> {
    Task {
      guard let url = await dependencies.chooseExportURL() else { return }
      do {
        try await dependencies.history.exportCSV(
          to: url, hidePersonalInformation: dependencies.settings.hidePersonalInformation)
        dependencies.log.log("history exported to \(url.lastPathComponent)")
      } catch {
        dependencies.log.logError("history export failed: \(error)")
      }
    }
  }

  @discardableResult
  public func clearHistory() -> Task<Void, Never> {
    Task {
      do {
        let removed = try await coordinator.clearHistory()
        dependencies.log.log("history cleared rows=\(removed)")
        environment.historyPresenter.invalidateData()
      } catch {
        dependencies.log.logError("history clear failed: \(error)")
      }
    }
  }

  public func revealHistory() {
    guard let location = dependencies.history.location else { return }
    dependencies.revealInFinder(location)
  }

  public func diagnosticsReport() -> String {
    Diagnostics.report(
      app: dependencies.appInfo,
      osVersion: ProcessInfo.processInfo.operatingSystemVersionString,
      settings: dependencies.settings,
      state: dependencies.state,
      historyLocation: dependencies.history.location,
      log: dependencies.log,
      now: dependencies.clock.now()
    )
  }

  public func copyDiagnostics() {
    dependencies.copyToPasteboard(diagnosticsReport())
  }

  public func reportIssue() {
    dependencies.openURL(
      Diagnostics.issueURL(
        repository: dependencies.appInfo.repository, title: "Issue report", report: diagnosticsReport()))
  }

  public func showFullLog() {
    popover?.close(restoringActivation: false)
    if logWindow == nil {
      logWindow = LogWindowController(log: dependencies.log, presentsWindow: dependencies.presentsWindows)
    }
    logWindow?.showWindow(nil)
  }

  public func setLaunchAtLogin(_ enabled: Bool) {
    environment.launchAtLoginStatus = dependencies.launchAtLogin.setEnabled(enabled)
    dependencies.log.log("launch at login \(enabled ? "on" : "off") -> \(environment.launchAtLoginStatus.rawValue)")
  }

  public func grantAccess(to resource: SandboxResource) async {
    guard let url = await dependencies.chooseDirectory(resource) else { return }
    do {
      let bookmark = try await Task.detached(priority: .userInitiated) {
        try SecurityScopedBookmarkClient.live.create(url)
      }.value
      dependencies.settings.setBookmark(bookmark, for: resource)
      dependencies.settings.flush()
      dependencies.log.log("\(resource.label) access granted: \(url.lastPathComponent)")
      replaceProviders(await dependencies.rebuildProviders(dependencies.settings))
      refreshNow()
    } catch {
      dependencies.log.logError("bookmark for \(resource.label) failed: \(error)")
    }
  }

  public func setDemoMode(_ enabled: Bool) {
    dependencies.settings.demoMode = enabled
    dependencies.settings.flush()
    dependencies.isDemo = enabled
    environment.isDemo = enabled
    dependencies.log.log("demo mode \(enabled ? "on" : "off"); relaunching")
    popover?.close(restoringActivation: false)
    dependencies.relaunch()
  }

  public func replaceProviders(_ registry: ProviderRegistry) {
    replaceProviderRegistry(registry)
    probeProviderHealth()
  }
}
