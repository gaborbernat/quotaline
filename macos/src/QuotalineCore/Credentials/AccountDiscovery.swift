import Foundation

func summarySource(_ source: CredentialSource, count: Int, showing label: String) -> CredentialSource {
  return CredentialSource(
    id: source.id, provider: source.provider, title: source.title,
    detail: count > 1 ? "\(count) accounts found; showing \(label)" : source.detail)
}
