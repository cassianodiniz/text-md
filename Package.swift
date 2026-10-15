// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "text-md",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "text-md", targets: ["TextMD"])],
    dependencies: [.package(url: "https://github.com/swiftlang/swift-markdown.git", exact: "0.5.0")],
    targets: [
        .systemLibrary(name: "CSQLite"),
        .target(name: "TextMDCore", dependencies: ["CSQLite", .product(name: "Markdown", package: "swift-markdown")], resources: [.process("Resources")]),
        .executableTarget(name: "TextMD", dependencies: ["TextMDCore"]),
        .testTarget(name: "TextMDCoreTests", dependencies: ["TextMDCore"], resources: [.process("Fixtures")]),
        .testTarget(name: "TextMDAppTests", dependencies: ["TextMD", "CSQLite"])
    ]
)
