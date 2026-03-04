import Foundation

#if os(Linux)
import Glibc
#else
import Darwin
#endif

public enum Destack {
    public static let backend = "swift"
    public static let version = "0.55.3"
}

public struct Client {
    private static let capi = CapiBindings.load()

    public init() {}

    public var backend: String {
        Destack.backend
    }

    public func capiAbiVersion() -> UInt32 {
        Client.capi?.abiVersion() ?? 0
    }

    public func capiIsAvailable() -> Bool {
        Client.capi?.isAvailable() ?? false
    }

    public func version() -> String {
        Client.capi?.version() ?? Destack.version
    }
}

private final class CapiBindings: @unchecked Sendable {
    typealias AbiVersionFunction = @convention(c) () -> UInt32
    typealias VersionFunction = @convention(c) () -> UnsafePointer<CChar>?
    typealias IsAvailableFunction = @convention(c) () -> Bool

    private let abiVersionFunction: AbiVersionFunction
    private let versionFunction: VersionFunction
    private let isAvailableFunction: IsAvailableFunction

    private init(
        abiVersionFunction: @escaping AbiVersionFunction,
        versionFunction: @escaping VersionFunction,
        isAvailableFunction: @escaping IsAvailableFunction
    ) {
        self.abiVersionFunction = abiVersionFunction
        self.versionFunction = versionFunction
        self.isAvailableFunction = isAvailableFunction
    }

    static func load() -> CapiBindings? {
        let explicitPath = ProcessInfo.processInfo.environment["DESTACK_CAPI_LIB"] ?? ""
        var candidates: [String] = []
        if !explicitPath.isEmpty {
            candidates.append(explicitPath)
        }
        #if os(macOS)
        candidates.append("libdestack_capi.dylib")
        #elseif os(Linux)
        candidates.append("libdestack_capi.so")
        #elseif os(Windows)
        candidates.append("destack_capi.dll")
        #endif

        for candidate in candidates {
            guard let handle = dlopen(candidate, RTLD_NOW | RTLD_LOCAL) else {
                continue
            }
            guard
                let abiVersionSymbol = dlsym(handle, "destack_capi_abi_version"),
                let versionSymbol = dlsym(handle, "destack_capi_version"),
                let isAvailableSymbol = dlsym(handle, "destack_capi_is_available")
            else {
                continue
            }

            let abiVersionFunction = unsafeBitCast(abiVersionSymbol, to: AbiVersionFunction.self)
            let versionFunction = unsafeBitCast(versionSymbol, to: VersionFunction.self)
            let isAvailableFunction = unsafeBitCast(isAvailableSymbol, to: IsAvailableFunction.self)
            return CapiBindings(
                abiVersionFunction: abiVersionFunction,
                versionFunction: versionFunction,
                isAvailableFunction: isAvailableFunction
            )
        }

        return nil
    }

    func abiVersion() -> UInt32 {
        abiVersionFunction()
    }

    func version() -> String {
        guard let value = versionFunction() else {
            return Destack.version
        }

        return String(cString: value)
    }

    func isAvailable() -> Bool {
        isAvailableFunction()
    }
}
