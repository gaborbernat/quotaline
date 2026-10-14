import Foundation
import QuotalineRust

extension ClaudeTranscriptReader {
  static func projectName(transcript url: URL, root: URL) -> String {
    claudeProjectName(transcript: url.standardizedFileURL.path, root: root.standardizedFileURL.path)
  }

  public static func projectName(encodedDirectory name: String) -> String {
    claudeEncodedProjectName(name: name)
  }
}
