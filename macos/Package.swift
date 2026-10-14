// swift-tools-version: 6.2
import Foundation
import PackageDescription

let nativeTests = ProcessInfo.processInfo.environment["QUOTALINE_TEST_DESKTOP"] == "github-hosted"
if nativeTests {
  guard ProcessInfo.processInfo.environment["GITHUB_ACTIONS"] == "true",
    ProcessInfo.processInfo.environment["RUNNER_ENVIRONMENT"] == "github-hosted"
  else {
    FileHandle.standardError.write(
      Data("Native tests require a GitHub-hosted desktop. Run just macos-test locally.\n".utf8))
    exit(1)
  }
}
let nativeTestSources = nativeTests || ProcessInfo.processInfo.environment["QUOTALINE_COMPILE_NATIVE_TESTS"] == "1"

let strict: [SwiftSetting] = [
  .swiftLanguageMode(.v6),
  .enableUpcomingFeature("ExistentialAny"),
  .treatAllWarnings(as: .error),
]

let package = Package(
  name: "Quotaline",
  platforms: [.macOS(.v14)],
  products: [
    .library(name: "QuotalineCore", targets: ["QuotalineCore"]),
    .library(name: "QuotalineRust", targets: ["QuotalineRust"]),
    .library(name: "QuotalineUI", targets: ["QuotalineUI"]),
    .library(name: "QuotalineWidgets", targets: ["QuotalineWidgets"]),
    .executable(name: "Quotaline", targets: ["Quotaline"]),
  ],
  targets: [
    .binaryTarget(name: "quotaline_macos_ffiFFI", path: "Generated/QuotalineRustFFI.xcframework"),
    .target(
      name: "QuotalineRust", dependencies: ["quotaline_macos_ffiFFI"], path: "src/QuotalineRust",
      // UniFFI's Swift callback bindings omit `any` on protocol existential types.
      swiftSettings: [.swiftLanguageMode(.v6), .treatAllWarnings(as: .error)]),
    .target(
      name: "QuotalineCore", dependencies: ["QuotalineRust"], path: "src/QuotalineCore", swiftSettings: strict),
    .target(
      name: "QuotalineUI",
      dependencies: ["QuotalineCore"],
      path: "src/QuotalineUI",
      resources: [.process("Resources")],
      swiftSettings: strict
    ),
    .target(
      name: "QuotalineWidgets", dependencies: ["QuotalineCore"], path: "src/QuotalineWidgets",
      swiftSettings: strict),
    .executableTarget(
      name: "Quotaline", dependencies: ["QuotalineUI"], path: "src/Quotaline", swiftSettings: strict),
    .target(
      name: "QuotalineTestSupport",
      dependencies: ["QuotalineCore", "QuotalineRust"],
      path: "tests/QuotalineTestSupport",
      swiftSettings: strict
    ),
    .testTarget(
      name: "QuotalineCoreTests",
      dependencies: ["QuotalineCore", "QuotalineRust", "QuotalineTestSupport"],
      path: "tests/QuotalineCoreTests",
      resources: [.copy("Fixtures")],
      swiftSettings: strict
    ),
    .testTarget(
      name: "QuotalineUITests",
      dependencies: ["QuotalineUI", "QuotalineTestSupport"]
        + (nativeTestSources ? [.target(name: "QuotalineNativeGuard")] : []),
      path: "tests/QuotalineUITests",
      exclude: nativeTestSources ? [] : ["Native"],
      swiftSettings: strict + (nativeTestSources ? [] : [.define("NONPRESENTING_TESTS")])
    ),
    .target(name: "QuotalineNativeGuard", path: "tests/QuotalineNativeGuard"),
    .testTarget(
      name: "QuotalineWidgetsTests",
      dependencies: ["QuotalineWidgets", "QuotalineTestSupport"],
      path: "tests/QuotalineWidgetsTests",
      swiftSettings: strict + (nativeTests ? [] : [.define("NONPRESENTING_TESTS")])
    ),
  ]
)
