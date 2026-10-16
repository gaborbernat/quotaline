import AppKit
import Darwin
import QuotalineTestSupport
import Testing
import UserNotifications

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func appDelegateLifecycle() throws {
  let (dependencies, _) = try makeDependencies()
  let controller = AppController(dependencies: dependencies)
  let delegate = AppDelegate(controller: controller)
  delegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))
  #expect(controller.statusItem != nil)
  #expect(!delegate.applicationShouldHandleReopen(NSApp, hasVisibleWindows: false))
  delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
  #expect(controller.statusItem == nil)
}

@Test @MainActor func deferredAppDelegateShowsTheStatusShellBeforeDependenciesFinish() async throws {
  prepareTestApp()
  let (dependencies, _) = try makeDependencies()
  let gate = DeferredDependencyGate()
  var failure: String?
  let delegate = DeferredAppDelegate(presentsWindows: false, persistsStatusItemPosition: false) {
    try await gate.wait()
  } failureHandler: {
    failure = $0
  }
  delegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))
  #expect(delegate.statusShellVisible)
  #expect(delegate.controller == nil)

  gate.resolve(dependencies)
  await waitUntil { delegate.controller != nil }
  #expect(!delegate.statusShellVisible)
  #expect(delegate.controller?.statusItem != nil)
  #expect(failure == nil)
  delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
}

@Test @MainActor func deferredAppDelegateDiscardsDependenciesReturnedAfterCancellation() async throws {
  let (dependencies, _) = try makeDependencies()
  let gate = DeferredDependencyGate()
  let delegate = DeferredAppDelegate(presentsWindows: false, persistsStatusItemPosition: false) {
    try await gate.wait()
  } failureHandler: { _ in
    Issue.record("cancelled loading reported a failure")
  }

  delegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))
  await waitUntil { gate.isWaiting }
  delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
  gate.resolve(dependencies)
  await waitUntil { !gate.isWaiting }
  await mainActorTurn()

  #expect(delegate.controller == nil)
  #expect(!delegate.statusShellVisible)
}

@Test @MainActor func deferredBootstrapDoesNotTouchStorageDuringConstruction() {
  let root = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-deferred-\(UUID().uuidString)")
  let support = root.appendingPathComponent("support")
  let delegate = AppRunner.bootstrapDeferred(
    distribution: .appStore, notificationCenter: nil, updater: nil, isSandboxed: false,
    paths: LiveDependencies.Paths(home: root, supportDirectory: support, environment: [:], userName: "tester"),
    defaults: testDefaults(), transport: NoNetworkTransport(),
    keychain: testKeychain, launchAtLogin: .inMemory())
  #expect(!FileManager.default.fileExists(atPath: support.path))
  #expect(delegate.controller == nil)
}

@Test @MainActor func appRunnerBootstrapsAgainstTemporaryDefaults() async throws {
  let root = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-runner-\(UUID().uuidString)")
  let paths = LiveDependencies.Paths(
    home: root, supportDirectory: root.appendingPathComponent("support"), environment: [:], userName: "tester")
  let delegate = try await AppRunner.bootstrap(
    distribution: .appStore, notificationCenter: nil, updater: FakeUpdater(), isSandboxed: false, paths: paths,
    defaults: testDefaults(), transport: NoNetworkTransport(),
    keychain: testKeychain, launchAtLogin: .inMemory())
  #expect(delegate.controller.dependencies.appInfo.isAppStore)
  #expect(delegate.controller.environment.canCheckForUpdates)
}

@MainActor
private final class DeferredDependencyGate {
  private var result: Result<AppDependencies, any Error>?
  private var continuation: CheckedContinuation<AppDependencies, any Error>?
  private(set) var isWaiting = false

  func wait() async throws -> AppDependencies {
    if let result { return try result.get() }
    isWaiting = true
    defer { isWaiting = false }
    return try await withCheckedThrowingContinuation { continuation = $0 }
  }

  func resolve(_ dependencies: AppDependencies) {
    if let continuation {
      self.continuation = nil
      continuation.resume(returning: dependencies)
    } else {
      result = .success(dependencies)
    }
  }
}
