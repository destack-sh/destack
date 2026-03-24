import Foundation
import RuntimeHostAppleBridgeC

/// One bridge string payload decode failure.
enum BridgeStringError: Error {
  /// The bridge string slice was invalid.
  case invalidStringSlice
}

/// Build one C string reference for one optional Swift string.
func withNativeStringRef<T>(
  _ value: String?,
  body: (DestackRustStringRef) -> T
) -> T {
  guard let value else {
    return body(DestackRustStringRef(data: nil, len: 0))
  }

  let bytes = Array(value.utf8)

  return bytes.withUnsafeBufferPointer { buffer in
    body(
      DestackRustStringRef(
        data: buffer.baseAddress,
        len: UInt32(buffer.count)
      )
    )
  }
}

/// Build one C string-slice reference for one Swift string array.
func withNativeStringSlice<T>(
  _ values: [String],
  body: (DestackRustStringSlice) -> T
) -> T {
  let storage = values.map { Array($0.utf8) }
  let refs = storage.map { bytes in
    bytes.withUnsafeBufferPointer { buffer in
      DestackRustStringRef(
        data: buffer.baseAddress,
        len: UInt32(buffer.count)
      )
    }
  }

  return refs.withUnsafeBufferPointer { buffer in
    body(
      DestackRustStringSlice(
        data: buffer.baseAddress,
        len: UInt32(buffer.count)
      )
    )
  }
}

/// Decode one bridge string reference into one Swift string.
func tryDecodeNativeString(
  _ value: DestackRustStringRef
) throws -> String {
  if value.len == 0 {
    return ""
  }

  guard let data = value.data else {
    throw BridgeStringError.invalidStringSlice
  }

  let bytes = UnsafeBufferPointer(start: data, count: Int(value.len))

  guard let decoded = String(bytes: bytes, encoding: .utf8) else {
    throw BridgeStringError.invalidStringSlice
  }

  return decoded
}

/// Decode one bridge string slice into one Swift string array.
func tryDecodeNativeStringSlice(
  _ values: DestackRustStringSlice
) throws -> [String] {
  if values.len == 0 {
    return []
  }

  guard let data = values.data else {
    throw BridgeStringError.invalidStringSlice
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return try buffer.map(tryDecodeNativeString)
}

/// Decode one native string reference into one Swift string.
func decodeNativeString(
  _ value: DestackRustStringRef
) -> String {
  guard let data = value.data else {
    return ""
  }

  let bytes = UnsafeBufferPointer(start: data, count: Int(value.len))

  return String(decoding: bytes, as: UTF8.self)
}

/// Decode one bridge string slice into one Swift string array.
func decodeNativeStringSlice(
  _ values: DestackRustStringSlice
) -> [String] {
  guard values.len != 0, let data = values.data else {
    return []
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.map(decodeNativeString)
}
