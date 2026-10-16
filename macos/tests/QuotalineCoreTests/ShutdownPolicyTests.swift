import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func shutdownPolicyReportsCompletedWork() async {
  #expect(await ShutdownPolicy.waitForCompletion(timeout: .seconds(3600)) {})
}

@Test func shutdownPolicyStopsWaitingAtTheDeadline() async {
  let completed = await ShutdownPolicy.waitForCompletion(
    timeout: .milliseconds(5), pollInterval: .milliseconds(1)
  ) {
    // Runs until the policy gives up and cancels it, so it can never finish before the deadline.
    try? await Task.sleep(for: .seconds(3600))
  }

  #expect(!completed)
}
