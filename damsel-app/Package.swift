// swift-tools-version: 5.10

import PackageDescription

let package = Package(
    name: "DamselApp",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .executable(name: "DamselWorkbench", targets: ["DamselWorkbench"]),
        .library(name: "DamselAppCore", targets: ["DamselAppCore"]),
    ],
    targets: [
        .target(name: "DamselAppCore"),
        .executableTarget(
            name: "DamselWorkbench",
            dependencies: ["DamselAppCore"]
        ),
        .testTarget(
            name: "DamselAppCoreTests",
            dependencies: ["DamselAppCore"]
        ),
    ]
)
