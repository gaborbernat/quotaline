import Foundation
import QuotalineTestSupport

@testable import QuotalineCore

struct NoNetworkTransport: HTTPTransport {
  func data(for _: URLRequest) async throws -> (Data, URLResponse) {
    throw URLError(.unsupportedURL)
  }
}
