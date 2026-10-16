import AppKit
import QuotalineCore

public enum SingleInstanceGuard {
  /// Brings the instance already running under this bundle identifier forward and reports whether this process
  /// should exit in its favour. A bare `swift run` has no bundle identifier and is left alone, and so is the
  /// replacement for the instance named in `replacing`, which is on its way out.
  public static func handOff(
    policy: LaunchPolicy, replacing: pid_t? = nil, bundleIdentifier: String? = Bundle.main.bundleIdentifier,
    currentProcessIdentifier: pid_t = ProcessInfo.processInfo.processIdentifier,
    runningApplications: (String) -> [NSRunningApplication] = NSRunningApplication.runningApplications(
      withBundleIdentifier:)
  ) -> Bool {
    guard policy.yieldsToRunningInstance, let bundleIdentifier,
      let running = runningApplications(bundleIdentifier).first(where: {
        $0.processIdentifier != currentProcessIdentifier
      }),
      running.processIdentifier != replacing
    else { return false }
    running.activate()
    return true
  }
}
