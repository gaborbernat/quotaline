import QuotalineRust

public struct GeminiOAuthClient: Sendable, Equatable {
  public let id: String
  public let secret: String

  public init(id: String, secret: String) {
    self.id = id
    self.secret = secret
  }

  init(_ client: OAuthClientRecord) {
    self.init(id: client.id, secret: client.secret)
  }
}
