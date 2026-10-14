import QuotalineRust

public enum ProjectIdentity {
  public static func anonymous(_ identity: String) -> String {
    QuotalineRust.anonymousProject(identity: identity)
  }

  public static func labels(_ identities: [String], private: Bool) -> [String: String] {
    QuotalineRust.projectLabels(identities: identities, hideIdentity: `private`)
  }
}
