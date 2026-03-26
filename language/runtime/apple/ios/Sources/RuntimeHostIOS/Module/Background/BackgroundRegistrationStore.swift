import Foundation
import RuntimeHostAppleCore

private let backgroundRegistrationsKey = "dev.destack.runtime.ios.background.registrations"

/// One durable iOS background registration record.
internal struct BackgroundTaskRecord: Codable {
  /// The stored task descriptor.
  let descriptor: RuntimeHostBackgroundTaskDescriptor

  /// The next regular recurring execution target when this task recurs.
  let nextRegularRunUnixNs: UInt64?
}

/// One background registration storage result.
internal struct BackgroundStoredRegistration {
  /// The active record for this identifier.
  let record: BackgroundTaskRecord

  /// Whether the scheduler should update this registration.
  let shouldSchedule: Bool
}

/// The registration store for one iOS background host.
@MainActor
internal final class BackgroundRegistrationStore {
  /// The user defaults backing store.
  private let defaults: UserDefaults

  /// The identifiers with registered launch handlers.
  private(set) var registeredIdentifiers: Set<String>

  /// The registered records by stable identifier.
  private(set) var registrations: [String: BackgroundTaskRecord]

  /// Create one background registration store.
  init(
    defaults: UserDefaults
  ) {
    // capture the persistent store and restore the durable registrations
    self.defaults = defaults
    self.registrations = Self.loadRegistrations(defaults)
    self.registeredIdentifiers = []
  }

  /// Return every descriptor sorted by stable identifier.
  func listDescriptors() -> [RuntimeHostBackgroundTaskDescriptor] {
    // return one stable listing sorted by identifier
    registrations
      .values
      .map(\.descriptor)
      .sorted(by: { $0.identifier < $1.identifier })
  }

  /// Resolve one record by stable identifier.
  func resolveRecord(
    _ identifier: String
  ) -> BackgroundTaskRecord? {
    registrations[identifier]
  }

  /// Resolve one descriptor by stable identifier.
  func resolveDescriptor(
    _ identifier: String
  ) -> RuntimeHostBackgroundTaskDescriptor? {
    registrations[identifier]?.descriptor
  }

  /// Persist one registration descriptor.
  func putRegistration(
    _ descriptor: RuntimeHostBackgroundTaskDescriptor,
    nextRegularRunUnixNs: UInt64?
  ) -> BackgroundStoredRegistration {
    // keep one existing registration when the conflict policy says so
    if let existingRecord = registrations[descriptor.identifier],
      descriptor.conflictPolicy == .keep
    {
      return BackgroundStoredRegistration(
        record: existingRecord,
        shouldSchedule: false
      )
    }

    let record = BackgroundTaskRecord(
      descriptor: descriptor,
      nextRegularRunUnixNs: nextRegularRunUnixNs
    )

    // store the registration under its stable identifier
    registrations[descriptor.identifier] = record

    // flush the durable registration snapshot
    storeRegistrations()

    return BackgroundStoredRegistration(
      record: record,
      shouldSchedule: true
    )
  }

  /// Persist one full registration record.
  func putRecord(
    _ record: BackgroundTaskRecord
  ) {
    registrations[record.descriptor.identifier] = record

    storeRegistrations()
  }

  /// Remove one registration descriptor.
  func removeRegistration(
    _ identifier: String
  ) -> BackgroundTaskRecord? {
    // remove the registration from the live map
    let record = registrations.removeValue(forKey: identifier)

    // flush the durable registration snapshot when it changed
    if record != nil {
      storeRegistrations()
    }

    return record
  }

  /// Return whether the launch handler for this identifier is already registered.
  func isHandlerRegistered(
    _ identifier: String
  ) -> Bool {
    registeredIdentifiers.contains(identifier)
  }

  /// Mark one handler identifier as registered.
  func markHandlerRegistered(
    _ identifier: String
  ) {
    registeredIdentifiers.insert(identifier)
  }

  /// Load every durable registration descriptor from user defaults.
  private static func loadRegistrations(
    _ defaults: UserDefaults
  ) -> [String: BackgroundTaskRecord] {
    // load the stored payload when one exists
    guard let data = defaults.data(forKey: backgroundRegistrationsKey) else {
      return [:]
    }

    // decode the payload into runtime descriptors
    guard
      let records = try? JSONDecoder().decode(
        [BackgroundTaskRecord].self,
        from: data
      )
    else {
      return [:]
    }

    // index the descriptors by stable identifier
    return Dictionary(
      uniqueKeysWithValues: records.map { record in
        (record.descriptor.identifier, record)
      }
    )
  }

  /// Persist every durable registration descriptor into user defaults.
  private func storeRegistrations() {
    // keep every durable registration in a stable order
    let descriptors =
      registrations
      .values
      .sorted(by: { left, right in left.descriptor.identifier < right.descriptor.identifier })

    // encode the durable registration snapshot
    guard let data = try? JSONEncoder().encode(descriptors) else {
      return
    }

    // write the snapshot into user defaults
    defaults.set(data, forKey: backgroundRegistrationsKey)
  }
}
