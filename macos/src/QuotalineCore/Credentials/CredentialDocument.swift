import Foundation
import QuotalineRust

func credentialPayloadDocument(
  _ data: Data, provider: CredentialFileProvider, malformed: String
) throws -> JSONValue? {
  do {
    guard try QuotalineRust.credentialPayloadUsable(provider: provider, payload: data) else { return nil }
    return decodeCredentialDocument(data)
  } catch StorageFileError.Malformed {
    throw CredentialStoreError.malformed(malformed)
  }
}

func credentialFileDocument(at url: URL, provider: CredentialFileProvider) throws -> JSONValue? {
  do {
    guard let data = try QuotalineRust.readProviderCredentialFile(provider: provider, path: url.path) else {
      return nil
    }
    return decodeCredentialDocument(data)
  } catch StorageFileError.Malformed {
    throw CredentialStoreError.malformed("\(url.lastPathComponent) is not JSON")
  }
}

private func decodeCredentialDocument(_ data: Data) -> JSONValue {
  try! JSONDecoder().decode(JSONValue.self, from: data)
}
