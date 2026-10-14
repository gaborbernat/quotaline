import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test(arguments: [
  (arguments: ["Quotaline"], environment: [String: String](), yields: true),
  (arguments: ["Quotaline", LaunchPolicy.relaunchedArgument], environment: [:], yields: false),
  (arguments: ["Quotaline", LaunchPolicy.verificationArgument], environment: [:], yields: false),
  (arguments: ["Quotaline"], environment: [LaunchPolicy.verificationEnvironmentKey: "1"], yields: false),
])
func launchPolicyYieldsToARunningInstanceOnlyForPlainLaunches(
  arguments: [String], environment: [String: String], yields: Bool
) {
  #expect(LaunchPolicy(arguments: arguments, environment: environment).yieldsToRunningInstance == yields)
}

@Test @MainActor func singleInstanceGuardActivatesTheOtherInstanceAndAsksThisOneToExit() {
  prepareTestApp()
  var lookups: [String] = []

  let handedOff = SingleInstanceGuard.handOff(
    policy: LaunchPolicy(arguments: ["Quotaline"], environment: [:]), bundleIdentifier: "tech.bernat.quotaline",
    currentProcessIdentifier: 0,
    runningApplications: {
      lookups.append($0)
      return [.current]
    })

  #expect(handedOff)
  #expect(lookups == ["tech.bernat.quotaline"])
}

@Test @MainActor func singleInstanceGuardIgnoresItself() {
  let handedOff = SingleInstanceGuard.handOff(
    policy: LaunchPolicy(arguments: ["Quotaline"], environment: [:]), bundleIdentifier: "tech.bernat.quotaline",
    runningApplications: { _ in [.current] })

  #expect(!handedOff)
}

@Test @MainActor func singleInstanceGuardLeavesUnbundledAndRelaunchedProcessesAlone() {
  var lookups = 0
  let find: (String) -> [NSRunningApplication] = { _ in
    lookups += 1
    return [.current]
  }

  #expect(
    !SingleInstanceGuard.handOff(
      policy: LaunchPolicy(arguments: ["Quotaline"], environment: [:]), bundleIdentifier: nil,
      currentProcessIdentifier: 0, runningApplications: find))
  #expect(
    !SingleInstanceGuard.handOff(
      policy: LaunchPolicy(arguments: ["Quotaline", LaunchPolicy.relaunchedArgument], environment: [:]),
      bundleIdentifier: "tech.bernat.quotaline", currentProcessIdentifier: 0, runningApplications: find))
  #expect(lookups == 0)
}

// A sandboxed relaunch arrives without `--relaunched`, so only the handshake tells the replacement to stay.
@Test @MainActor func singleInstanceGuardLetsTheReplacementOfASandboxedRelaunchStart() {
  let handedOff = SingleInstanceGuard.handOff(
    policy: LaunchPolicy(arguments: ["Quotaline"], environment: [:]),
    replacing: NSRunningApplication.current.processIdentifier, bundleIdentifier: "tech.bernat.quotaline",
    currentProcessIdentifier: 0, runningApplications: { _ in [.current] })

  #expect(!handedOff)
}

@Test @MainActor func singleInstanceGuardConsultsTheRunningApplicationList() {
  let policy = LaunchPolicy(arguments: ["Quotaline"], environment: [:])

  #expect(!SingleInstanceGuard.handOff(policy: policy, bundleIdentifier: "tech.bernat.quotaline.absent"))
}
