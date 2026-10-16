import Foundation
import QuotalineRust

public struct SandboxResource: Sendable, Hashable, Identifiable {
  public enum Need: Sendable, Hashable {
    case required
    case optional
    case oneOf(String)
  }

  public let id: String
  public let relativePath: String
  public let provider: ProviderID
  public let need: Need
  public let explanation: String

  fileprivate init(_ record: SandboxResourceRecord) {
    id = record.id
    relativePath = record.relativePath
    provider = ProviderID(rawValue: record.provider)!
    need =
      switch record.need {
      case .required: .required
      case .optional: .optional
      case .oneOf(let group): .oneOf(group)
      }
    explanation = record.explanation
  }

  public var alternativesGroup: String? {
    if case .oneOf(let group) = need { group } else { nil }
  }

  public var label: String {
    "~/\(relativePath)"
  }

  public func configuredURL(environment: [String: String], home: URL) -> URL {
    URL(
      fileURLWithPath: QuotalineRust.configuredSandboxResourcePath(
        id: id, environment: environment, home: home.path), isDirectory: true)
  }
}

extension ProviderID {
  private static let resources = QuotalineRust.sandboxResourceDescriptors().map(SandboxResource.init)

  public var sandboxResources: [SandboxResource] {
    Self.resources.filter { $0.provider == self }
  }

  public static var allSandboxResources: [SandboxResource] {
    resources
  }
}
