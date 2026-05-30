import Foundation

public enum DamselCommandID: String, CaseIterable, Codable, Sendable {
    case openBinary = "app.open_binary"
    case showCommandPalette = "app.show_command_palette"
    case revealBinary = "app.reveal_binary"
    case copyBinaryPath = "app.copy_binary_path"
    case runDisassembly = "analysis.disassemble"
    case refreshDisassembly = "analysis.refresh_disassembly"
    case cancelAnalysis = "analysis.cancel"
    case navigateBack = "navigation.back"
    case navigateForward = "navigation.forward"
    case focusInstructionFilter = "navigation.focus_instruction_filter"
    case clearInstructionFilter = "navigation.clear_instruction_filter"
    case copySelectedAddress = "edit.copy_selected_address"
    case copySelectedInstruction = "edit.copy_selected_instruction"
    case copySelectedRow = "edit.copy_selected_row"
    case copyDiagnostics = "help.copy_diagnostics"
}

public struct DamselCommandShortcut: Equatable, Sendable {
    public enum Modifier: String, CaseIterable, Sendable {
        case command
        case option
        case shift
        case control
    }

    public let keyEquivalent: String
    public let modifiers: Set<Modifier>

    public init(_ keyEquivalent: String, modifiers: Set<Modifier> = [.command]) {
        self.keyEquivalent = keyEquivalent
        self.modifiers = modifiers
    }
}

public struct DamselCommandDescriptor: Equatable, Identifiable, Sendable {
    public let id: DamselCommandID
    public let title: String
    public let menuPath: [String]
    public let shortcut: DamselCommandShortcut?
    private let notificationNameRawValue: String?

    public init(
        id: DamselCommandID,
        title: String,
        menuPath: [String],
        shortcut: DamselCommandShortcut? = nil,
        notificationName: Notification.Name? = nil
    ) {
        self.id = id
        self.title = title
        self.menuPath = menuPath
        self.shortcut = shortcut
        notificationNameRawValue = notificationName?.rawValue
    }

    public var notificationName: Notification.Name? {
        notificationNameRawValue.map { Notification.Name(rawValue: $0) }
    }
}

public struct DamselCommandAvailability: Equatable, Sendable {
    public let isEnabled: Bool
    public let disabledReason: String?

    public init(isEnabled: Bool, disabledReason: String? = nil) {
        self.isEnabled = isEnabled
        self.disabledReason = isEnabled ? nil : disabledReason
    }

    public static let enabled = DamselCommandAvailability(isEnabled: true)

    public static func disabled(_ reason: String) -> DamselCommandAvailability {
        DamselCommandAvailability(isEnabled: false, disabledReason: reason)
    }
}

public enum DamselCommandRegistry {
    public static let all: [DamselCommandDescriptor] = [
        DamselCommandDescriptor(
            id: .openBinary,
            title: "Open Binary...",
            menuPath: ["File"],
            shortcut: DamselCommandShortcut("o"),
            notificationName: .damselOpenBinaryCommand
        ),
        DamselCommandDescriptor(
            id: .showCommandPalette,
            title: "Show Command Palette",
            menuPath: ["Navigate"],
            shortcut: DamselCommandShortcut("p", modifiers: [.command, .shift]),
            notificationName: .damselShowCommandPaletteCommand
        ),
        DamselCommandDescriptor(
            id: .revealBinary,
            title: "Reveal Binary in Finder",
            menuPath: ["File"]
        ),
        DamselCommandDescriptor(
            id: .copyBinaryPath,
            title: "Copy Binary Path",
            menuPath: ["File"]
        ),
        DamselCommandDescriptor(
            id: .runDisassembly,
            title: "Disassemble",
            menuPath: ["Analyze"],
            shortcut: DamselCommandShortcut("\r"),
            notificationName: .damselRunDisassemblyCommand
        ),
        DamselCommandDescriptor(
            id: .refreshDisassembly,
            title: "Refresh Disassembly",
            menuPath: ["Analyze"],
            shortcut: DamselCommandShortcut("r")
        ),
        DamselCommandDescriptor(
            id: .cancelAnalysis,
            title: "Cancel Analysis",
            menuPath: ["Analyze"],
            shortcut: DamselCommandShortcut(".")
        ),
        DamselCommandDescriptor(
            id: .navigateBack,
            title: "Back",
            menuPath: ["Navigate"],
            shortcut: DamselCommandShortcut("[")
        ),
        DamselCommandDescriptor(
            id: .navigateForward,
            title: "Forward",
            menuPath: ["Navigate"],
            shortcut: DamselCommandShortcut("]")
        ),
        DamselCommandDescriptor(
            id: .focusInstructionFilter,
            title: "Find in Instructions",
            menuPath: ["Navigate"],
            shortcut: DamselCommandShortcut("f"),
            notificationName: .damselFocusInstructionFilterCommand
        ),
        DamselCommandDescriptor(
            id: .clearInstructionFilter,
            title: "Clear Instruction Filter",
            menuPath: ["Navigate"],
            shortcut: DamselCommandShortcut("f", modifiers: [.command, .shift]),
            notificationName: .damselClearInstructionFilterCommand
        ),
        DamselCommandDescriptor(
            id: .copySelectedAddress,
            title: "Copy Selected Address",
            menuPath: ["Edit"]
        ),
        DamselCommandDescriptor(
            id: .copySelectedInstruction,
            title: "Copy Selected Instruction",
            menuPath: ["Edit"]
        ),
        DamselCommandDescriptor(
            id: .copySelectedRow,
            title: "Copy Selected Row",
            menuPath: ["Edit"]
        ),
        DamselCommandDescriptor(
            id: .copyDiagnostics,
            title: "Copy Diagnostics",
            menuPath: ["Help"],
            shortcut: DamselCommandShortcut("d", modifiers: [.command, .option])
        ),
    ]

    public static func descriptor(for id: DamselCommandID) -> DamselCommandDescriptor {
        guard let descriptor = all.first(where: { $0.id == id }) else {
            preconditionFailure("Missing Damsel command descriptor for \(id.rawValue)")
        }
        return descriptor
    }
}

public extension Notification.Name {
    static let damselOpenBinaryCommand = Notification.Name("DamselOpenBinaryCommand")
    static let damselShowCommandPaletteCommand = Notification.Name("DamselShowCommandPaletteCommand")
    static let damselRunDisassemblyCommand = Notification.Name("DamselRunDisassemblyCommand")
    static let damselFocusInstructionFilterCommand = Notification.Name("DamselFocusInstructionFilterCommand")
    static let damselClearInstructionFilterCommand = Notification.Name("DamselClearInstructionFilterCommand")
}
