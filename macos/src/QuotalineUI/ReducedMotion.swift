/// Set only by a verification launch that asks for it; every other launch follows the system Reduce Motion setting.
@MainActor enum ReducedMotion {
  static var forced = false
}
