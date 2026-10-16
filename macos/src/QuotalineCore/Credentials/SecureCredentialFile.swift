import Foundation
import QuotalineRust

func writeCredentialData(_ data: Data, to url: URL) throws {
  try QuotalineRust.writeSecretFile(path: url.path, data: data)
}
