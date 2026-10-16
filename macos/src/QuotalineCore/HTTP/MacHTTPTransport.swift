import Foundation
import QuotalineRust

private enum NativeRequest: Sendable {
  case get(NativeHttpGet)
  case postJSON(NativeHttpPostJson)
  case postForm(NativeHttpPostForm)
}

final class MacHTTPTransport: NativeHttpTransport {
  private let client: APIClient

  init(client: APIClient) {
    self.client = client
  }

  func startGet(request: NativeHttpGet, completion: NativeHttpCompletion) {
    perform(.get(request), completion: completion)
  }

  func startPostJson(request: NativeHttpPostJson, completion: NativeHttpCompletion) {
    perform(.postJSON(request), completion: completion)
  }

  func startPostForm(request: NativeHttpPostForm, completion: NativeHttpCompletion) {
    perform(.postForm(request), completion: completion)
  }

  private func perform(_ request: NativeRequest, completion: NativeHttpCompletion) {
    Task {
      switch await execute(request) {
      case .success(let data):
        completion.succeed(data: data)
      case .failure(let error):
        completion.fail(error: error.rust)
      }
    }
  }

  private func execute(_ request: NativeRequest) async -> Result<Data, APIError> {
    do {
      let data: Data
      switch request {
      case .get(let get):
        data = try await client.get(URL(string: get.url)!, headers: get.headers, operation: get.operation)
      case .postJSON(let post):
        data = try await client.post(
          URL(string: post.url)!, json: post.body, headers: post.headers, operation: post.operation,
          timeout: post.timeoutMillis.map { TimeInterval($0) / 1_000 } ?? APIClient.timeout)
      case .postForm(let post):
        data = try await client.post(
          URL(string: post.url)!, form: post.fields, headers: post.headers, operation: post.operation)
      }
      return .success(data)
    } catch {
      return .failure(error)
    }
  }
}
