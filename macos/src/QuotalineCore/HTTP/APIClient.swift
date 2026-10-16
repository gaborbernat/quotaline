import Foundation
import QuotalineRust

public protocol HTTPTransport: Sendable {
  func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: HTTPTransport {}

public struct DisabledHTTPTransport: HTTPTransport {
  public init() {}

  public func data(for _: URLRequest) async throws -> (Data, URLResponse) {
    throw URLError(.unsupportedURL)
  }
}

/// Antigravity's language server answers on the loopback interface with a self-signed certificate. Trust is extended
/// to that interface alone; every other host stays on the system trust store.
public enum LoopbackTrust {
  public static let hosts: Set<String> = ["127.0.0.1", "localhost", "::1"]

  public static func accepts(host: String, authenticationMethod: String) -> Bool {
    authenticationMethod == NSURLAuthenticationMethodServerTrust && hosts.contains(host)
  }
}

public enum APIError: Error, Equatable, Sendable {
  case http(status: Int, body: String, retryAfter: TimeInterval?)
  case vendorSecurityCheck(status: Int)
  case network(String)
  case decoding(String)

  public var isAuthenticationFailure: Bool {
    guard case .http(let status, let body, _) = self else { return false }
    return QuotalineRust.httpAuthenticationFailure(status: UInt16(clamping: status), body: Data(body.utf8))
  }

  public var isRateLimited: Bool {
    if case .http(let status, _, _) = self { return status == 429 }
    return false
  }

  public var retryAfter: TimeInterval? {
    if case .http(_, _, let retryAfter) = self { return retryAfter }
    return nil
  }

  public var message: String {
    QuotalineRust.httpErrorMessage(
      error: rust, home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path)
  }

  var rust: HttpErrorRecord {
    switch self {
    case .http(let status, let body, let retryAfter):
      .response(status: Int32(clamping: status), body: body, retryAfter: retryAfter)
    case .vendorSecurityCheck(let status): .vendorSecurityCheck(status: Int32(clamping: status))
    case .network(let detail): .network(detail: detail)
    case .decoding(let detail): .decoding(detail: detail)
    }
  }

  init(_ error: HttpErrorRecord) {
    switch error {
    case .response(let status, let body, let retryAfter):
      self = .http(status: Int(status), body: body, retryAfter: retryAfter)
    case .vendorSecurityCheck(let status): self = .vendorSecurityCheck(status: Int(status))
    case .network(let detail): self = .network(detail)
    case .decoding(let detail): self = .decoding(detail)
    }
  }
}

public struct APIClient: Sendable {
  public static let timeout: TimeInterval = 20
  public static let resourceTimeout: TimeInterval = 60

  private let transport: any HTTPTransport
  private let log: LogBuffer
  private let clock: Clock
  private let decoder: JSONDecoder

  public init(transport: any HTTPTransport, log: LogBuffer, clock: Clock = .system) {
    self.transport = transport
    self.log = log
    self.clock = clock
    decoder = JSONDecoder()
  }

  static func liveConfiguration() -> URLSessionConfiguration {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.httpCookieStorage = nil
    configuration.urlCredentialStorage = nil
    configuration.urlCache = nil
    configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
    configuration.timeoutIntervalForRequest = timeout
    configuration.timeoutIntervalForResource = resourceTimeout
    return configuration
  }

  public func get(_ url: URL, headers: [String: String], operation: String) async throws(APIError) -> Data {
    let request = URLRequest(url: url, timeoutInterval: Self.timeout)
    return try await send(request, method: "GET", headers: headers, operation: operation)
  }

  public func post(
    _ url: URL, json body: Data, headers: [String: String], operation: String, timeout: TimeInterval = Self.timeout
  ) async throws(APIError) -> Data {
    var request = URLRequest(url: url, timeoutInterval: timeout)
    request.httpBody = body
    return try await send(
      request, method: "POST", headers: headers.merging(["Content-Type": "application/json"]) { $1 },
      operation: operation)
  }

  public func post(
    _ url: URL, form fields: [String: String], headers: [String: String], operation: String
  ) async throws(APIError) -> Data {
    var request = URLRequest(url: url, timeoutInterval: Self.timeout)
    var components = URLComponents()
    components.queryItems = fields.keys.sorted().map { URLQueryItem(name: $0, value: fields[$0]) }
    // Assigning queryItems, even an empty array, makes the query non-nil.
    request.httpBody = Data(components.percentEncodedQuery!.replacingOccurrences(of: "+", with: "%2B").utf8)
    return try await send(
      request, method: "POST",
      headers: headers.merging(["Content-Type": "application/x-www-form-urlencoded"]) { $1 },
      operation: operation)
  }

  public func getJSON<Payload: Decodable>(
    _ type: Payload.Type, _ url: URL, headers: [String: String], operation: String
  ) async throws(APIError) -> Payload {
    let data = try await get(url, headers: headers, operation: operation)
    return try decode(type, data, operation: operation)
  }

  public func decode<Payload: Decodable>(
    _ type: Payload.Type, _ data: Data, operation: String
  ) throws(APIError) -> Payload {
    do {
      return try decoder.decode(type, from: data)
    } catch {
      log.logDebug(
        "decode failed operation=\(operation) \(Self.decodingDiagnostic(error))",
        category: .network)
      throw APIError.decoding("\(operation): invalid payload")
    }
  }

  private func send(
    _ base: URLRequest, method: String, headers: [String: String], operation: String
  ) async throws(APIError) -> Data {
    var request = base
    request.httpMethod = method
    request.cachePolicy = .reloadIgnoringLocalCacheData
    request.setValue("application/json", forHTTPHeaderField: "Accept")
    for (key, value) in headers { request.setValue(value, forHTTPHeaderField: key) }
    let id = String(UUID().uuidString.prefix(8)).lowercased()
    let endpoint = Self.redact(request.url)
    let started = clock.now()
    log.logDebug(
      "request started operation=\(operation) id=\(id) method=\(method) endpoint=\(endpoint)",
      category: .network)
    let data: Data
    let response: URLResponse
    do {
      (data, response) = try await transport.data(for: request)
    } catch {
      let duration = Int(clock.now().timeIntervalSince(started) * 1000)
      let value = error as NSError
      log.detailed(
        .request(
          RequestDiagnostic(
            requestID: id,
            operation: operation,
            method: method,
            byteCount: 0,
            durationMilliseconds: duration,
            error: error)))
      log.logDebug(
        "request failed operation=\(operation) id=\(id) endpoint=\(endpoint) "
          + "errorDomain=\(value.domain) errorCode=\(value.code)",
        category: .network)
      throw APIError.network(error.localizedDescription)
    }
    let status = (response as? HTTPURLResponse)?.statusCode ?? 0
    let duration = Int(clock.now().timeIntervalSince(started) * 1000)
    log.detailed(
      .request(
        RequestDiagnostic(
          requestID: id,
          operation: operation,
          method: method,
          status: status,
          byteCount: data.count,
          durationMilliseconds: duration)))
    guard
      let failure = QuotalineRust.httpResponseFailure(
        status: UInt16(clamping: status),
        contentType: (response as? HTTPURLResponse)?.value(forHTTPHeaderField: "Content-Type"),
        retryHeader: (response as? HTTPURLResponse)?.value(forHTTPHeaderField: "Retry-After"),
        body: data, now: clock.now().timeIntervalSince1970)
    else { return data }
    log.logDebug(
      "request rejected operation=\(operation) id=\(id) status=\(status) bytes=\(data.count) "
        + "duration=\(duration)ms",
      category: .network)
    if failure.securityCheck {
      throw APIError.vendorSecurityCheck(status: status)
    }
    throw APIError.http(status: status, body: failure.body, retryAfter: failure.retryAfter)
  }

  private static func decodingDiagnostic(_ error: any Error) -> String {
    let context: DecodingError.Context
    let path: [any CodingKey]
    switch error {
    case DecodingError.keyNotFound(let key, let detail):
      context = detail
      path = detail.codingPath + [key]
    case DecodingError.dataCorrupted(let detail), DecodingError.typeMismatch(_, let detail),
      DecodingError.valueNotFound(_, let detail):
      context = detail
      path = detail.codingPath
    default:
      return LogSanitizer.message(String(describing: error))
    }
    return LogSanitizer.message(
      "Path: \(path.isEmpty ? "<root>" : path.map(\.stringValue).joined(separator: ".")); \(context.debugDescription)")
  }

  static func redirectedRequest(from source: URL?, to request: URLRequest) -> URLRequest? {
    guard let source, let destination = request.url else { return nil }
    guard
      QuotalineRust.httpSameOrigin(
        source: HttpOrigin(scheme: source.scheme, host: source.host, port: source.port.map(Int32.init)),
        destination: HttpOrigin(
          scheme: destination.scheme, host: destination.host, port: destination.port.map(Int32.init)))
    else { return nil }
    return request
  }

  static func redact(_ url: URL?) -> String {
    guard let url, var components = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return "-" }
    components.query = nil
    return QuotalineRust.httpRedactedEndpoint(value: components.string!)
  }
}
