import Foundation

public enum ServerState: Equatable, Sendable {
    case stopped
    case starting
    case ready(URL)
    case failed(String)

    public var displayText: String {
        switch self {
        case .stopped:
            return "Server stopped"
        case .starting:
            return "Starting Damsel server"
        case let .ready(url):
            return "Ready at \(url.absoluteString)"
        case let .failed(message):
            return "Server failed: \(message)"
        }
    }
}

public enum DamselServerError: LocalizedError, Equatable, Sendable {
    case executableNotFound
    case launchFailed(String)
    case startupTimedOut(String)
    case terminatedBeforeReady(String)

    public var errorDescription: String? {
        switch self {
        case .executableNotFound:
            return "Could not find the damsel CLI. Set DAMSEL_CLI_PATH or launch the bundled app."
        case let .launchFailed(message):
            return "Could not launch the Damsel server: \(message)"
        case let .startupTimedOut(message):
            return "The Damsel server did not become ready in time. \(message)"
        case let .terminatedBeforeReady(message):
            return "The Damsel server exited before it reported a URL. \(message)"
        }
    }
}

protocol DamselServerStarting: AnyObject {
    func start() async throws -> URL
    func stop()
}

public final class DamselServerProcess {
    private var process: Process?
    private var readyURL: URL?

    public init() {}

    deinit {
        stop()
    }

    public func start() async throws -> URL {
        if let externalURL = externalBaseURL() {
            readyURL = externalURL
            return externalURL
        }

        if let readyURL, process?.isRunning == true {
            return readyURL
        }

        let executable = try resolveExecutable()
        let process = Process()
        process.executableURL = executable
        process.arguments = ["ui", "--no-open", "--port", "0"]

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw DamselServerError.launchFailed(error.localizedDescription)
        }

        self.process = process

        do {
            let url = try await waitForServerURL(process: process, stdout: stdout, stderr: stderr)
            readyURL = url
            return url
        } catch {
            process.terminate()
            throw error
        }
    }

    public func stop() {
        guard let process else {
            return
        }
        if process.isRunning {
            process.terminate()
        }
        self.process = nil
        readyURL = nil
    }

    private func waitForServerURL(process: Process, stdout: Pipe, stderr: Pipe) async throws -> URL {
        try await withCheckedThrowingContinuation { continuation in
            let stdoutHandle = stdout.fileHandleForReading
            let stderrHandle = stderr.fileHandleForReading
            let coordinator = ServerStartupCoordinator(
                process: process,
                stdoutHandle: stdoutHandle,
                stderrHandle: stderrHandle,
                continuation: continuation
            )

            stdoutHandle.readabilityHandler = { handle in
                let chunk = handle.availableData
                guard !chunk.isEmpty else {
                    return
                }
                coordinator.appendStdout(chunk)
            }

            process.terminationHandler = { _ in
                coordinator.failFromTermination()
            }

            DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + 10) {
                coordinator.failFromTimeout()
            }
        }
    }

    private func externalBaseURL() -> URL? {
        guard let rawValue = ProcessInfo.processInfo.environment["DAMSEL_UI_BASE_URL"],
              let url = URL(string: rawValue),
              url.scheme?.hasPrefix("http") == true
        else {
            return nil
        }
        return url
    }

    private func resolveExecutable() throws -> URL {
        let environment = ProcessInfo.processInfo.environment
        if let path = environment["DAMSEL_CLI_PATH"], !path.isEmpty {
            let url = URL(fileURLWithPath: path)
            if FileManager.default.isExecutableFile(atPath: url.path) {
                return url
            }
        }

        let bundleCandidates = [
            Bundle.main.resourceURL?.appendingPathComponent("damsel-cli"),
            Bundle.main.executableURL?
                .deletingLastPathComponent()
                .deletingLastPathComponent()
                .appendingPathComponent("Resources/damsel-cli"),
        ].compactMap { $0 }

        for candidate in bundleCandidates where FileManager.default.isExecutableFile(atPath: candidate.path) {
            return candidate
        }

        let workingDirectory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let repoCandidates = [
            workingDirectory.appendingPathComponent("target/debug/damsel-cli"),
            workingDirectory.appendingPathComponent("target/release/damsel-cli"),
            workingDirectory.deletingLastPathComponent().appendingPathComponent("target/debug/damsel-cli"),
        ]

        for candidate in repoCandidates where FileManager.default.isExecutableFile(atPath: candidate.path) {
            return candidate
        }

        throw DamselServerError.executableNotFound
    }

    static func parseReadyURL(from text: String) -> URL? {
        for line in text.split(whereSeparator: \.isNewline) {
            guard let range = line.range(of: "http://") ?? line.range(of: "https://") else {
                continue
            }
            let candidate = line[range.lowerBound...]
                .split(whereSeparator: \.isWhitespace)
                .first
                .map(String.init)
            if let candidate, let url = URL(string: candidate) {
                return url
            }
        }
        return nil
    }
}

extension DamselServerProcess: DamselServerStarting {}

private final class ServerStartupCoordinator: @unchecked Sendable {
    private let process: Process
    private let stdoutHandle: FileHandle
    private let stderrHandle: FileHandle
    private let continuation: CheckedContinuation<URL, Error>
    private let lock = NSLock()
    private var output = Data()
    private var completed = false

    init(
        process: Process,
        stdoutHandle: FileHandle,
        stderrHandle: FileHandle,
        continuation: CheckedContinuation<URL, Error>
    ) {
        self.process = process
        self.stdoutHandle = stdoutHandle
        self.stderrHandle = stderrHandle
        self.continuation = continuation
    }

    func appendStdout(_ chunk: Data) {
        lock.lock()
        output.append(chunk)
        let text = String(data: output, encoding: .utf8)
        lock.unlock()

        guard let text, let url = DamselServerProcess.parseReadyURL(from: text) else {
            return
        }
        finish(.success(url))
    }

    func failFromTermination() {
        let stderrText = String(data: stderrHandle.availableData, encoding: .utf8) ?? ""
        finish(.failure(DamselServerError.terminatedBeforeReady(stderrText)))
    }

    func failFromTimeout() {
        finish(.failure(DamselServerError.startupTimedOut("No readiness banner arrived on stdout.")))
    }

    private func finish(_ result: Result<URL, Error>) {
        lock.lock()
        defer { lock.unlock() }
        guard !completed else {
            return
        }
        completed = true
        stdoutHandle.readabilityHandler = nil
        stderrHandle.readabilityHandler = nil
        process.terminationHandler = nil

        switch result {
        case let .success(url):
            continuation.resume(returning: url)
        case let .failure(error):
            continuation.resume(throwing: error)
        }
    }
}
