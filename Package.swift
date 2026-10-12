// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "text-md",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "text-md", targets: ["TextMD"])],
    targets: [
        .systemLibrary(name: "CSQLite"),
        .target(name: "TextMDCore", dependencies: ["CSQLite"], resources: [.process("Resources")]),
        .executableTarget(name: "TextMD", dependencies: ["TextMDCore"]),
        .testTarget(name: "TextMDCoreTests", dependencies: ["TextMDCore"], resources: [.process("Fixtures")])
    ]
)
