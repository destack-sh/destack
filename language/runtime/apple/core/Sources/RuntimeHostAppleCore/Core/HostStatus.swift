import Foundation

/// The host status code for one successful host call.
public let hostStatusOk: UInt32 = 0
/// The host status code for one unsupported host call.
public let hostStatusNotSupported: UInt32 = 1
/// The host status code for one invalid host argument.
public let hostStatusInvalidArgument: UInt32 = 2
/// The host status code for one missing host binding.
public let hostStatusNotFound: UInt32 = 3
/// The host status code for one denied host permission.
public let hostStatusPermissionDenied: UInt32 = 4
/// The host status code for one buffer that was too small.
public let hostStatusBufferTooSmall: UInt32 = 5
/// The host status code for one generic host failure.
public let hostStatusFailed: UInt32 = 6

/// The host status code returned by one callback registration or request submission.
public typealias HostAbiStatus = UInt32
