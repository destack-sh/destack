package dev.destack.runtime.android.module.background

import android.content.SharedPreferences

import org.json.JSONArray
import org.json.JSONObject

internal const val backgroundPreferencesName: String = "dev.destack.runtime.background"

private const val backgroundRecordsKey: String = "records"

/**
 * One durable Android background task record.
 */
internal data class BackgroundTaskRecord(
    /**
     * The stored task descriptor.
     */
    val descriptor: RuntimeHostBackgroundTaskDescriptor,

    /**
     * The next regular recurring execution target when this task recurs.
     */
    val nextRegularRunUnixNs: Long?,
)

/**
 * One background registration storage result.
 */
internal data class BackgroundStoredRegistration(
    /**
     * The active record for the identifier.
     */
    val record: BackgroundTaskRecord,

    /**
     * Whether the scheduler should update this registration.
     */
    val shouldSchedule: Boolean,
)

/**
 * The task registration store for one Android process.
 */
internal class BackgroundRegistrationStore(
    private val preferences: SharedPreferences,
) {
    /**
     * The durable records loaded from shared preferences.
     */
    private val records: MutableMap<String, BackgroundTaskRecord> =
        loadRecords(preferences)

    /**
     * Return every registered task descriptor.
     */
    fun listDescriptors(): List<RuntimeHostBackgroundTaskDescriptor> {
        return records
            .values
            .map { record -> record.descriptor }
            .sortedBy { descriptor -> descriptor.identifier }
    }

    /**
     * Resolve one record by stable identifier.
     */
    fun resolveRecord(
        identifier: String,
    ): BackgroundTaskRecord? {
        return records[identifier]
    }

    /**
     * Store one registration payload and report whether the scheduler should update it.
     */
    fun putRegistration(
        options: RuntimeHostBackgroundTaskOptions,
        nextRegularRunUnixNs: Long?,
    ): BackgroundStoredRegistration {
        val existingRecord = records[options.identifier]
        if (
            existingRecord != null &&
            options.conflictPolicy == RuntimeHostBackgroundConflictPolicy.Keep
        ) {
            return BackgroundStoredRegistration(
                record = existingRecord,
                shouldSchedule = false,
            )
        }

        val record = BackgroundTaskRecord(
            descriptor = descriptorFromOptions(options),
            nextRegularRunUnixNs = nextRegularRunUnixNs,
        )

        records[options.identifier] = record

        storeRecords(preferences, records.values)

        return BackgroundStoredRegistration(
            record = record,
            shouldSchedule = true,
        )
    }

    /**
     * Remove one registration by stable identifier.
     */
    fun removeRegistration(
        identifier: String,
    ): BackgroundTaskRecord? {
        val record = records.remove(identifier)

        if (record != null) {
            storeRecords(preferences, records.values)
        }

        return record
    }

    /**
     * Replace one stored record under its stable identifier.
     */
    fun putRecord(
        record: BackgroundTaskRecord,
    ) {
        records[record.descriptor.identifier] = record

        storeRecords(preferences, records.values)
    }
}

/**
 * Build one task descriptor view from one registration payload.
 */
private fun descriptorFromOptions(
    options: RuntimeHostBackgroundTaskOptions,
): RuntimeHostBackgroundTaskDescriptor {
    return RuntimeHostBackgroundTaskDescriptor(
        identifier = options.identifier,
        trigger = options.trigger,
        schedule = options.schedule,
        network = options.network,
        requiresCharging = options.requiresCharging,
        requiresIdle = options.requiresIdle,
        conflictPolicy = options.conflictPolicy,
    )
}

/**
 * Load every durable background record from shared preferences.
 */
private fun loadRecords(
    preferences: SharedPreferences,
): MutableMap<String, BackgroundTaskRecord> {
    val payload = preferences.getString(backgroundRecordsKey, null)
        ?: return mutableMapOf()
    val entries = JSONArray(payload)
    val records = mutableMapOf<String, BackgroundTaskRecord>()

    for (index in 0 until entries.length()) {
        val entry = entries.getJSONObject(index)
        val descriptor = RuntimeHostBackgroundTaskDescriptor(
            identifier = entry.getString("identifier"),
            trigger = decodeTrigger(entry.getInt("trigger")) ?: continue,
            schedule = RuntimeHostBackgroundTaskSchedule(
                kind = decodeScheduleKind(entry.getInt("scheduleKind")) ?: continue,
                earliestBeginUnixNs =
                    if (entry.isNull("earliestBeginUnixNs")) null else entry.getLong(
                        "earliestBeginUnixNs",
                    ),
                repeatIntervalNs =
                    if (entry.isNull("repeatIntervalNs")) null else entry.getLong(
                        "repeatIntervalNs",
                    ),
            ),
            network = decodeNetwork(entry.getInt("network")) ?: continue,
            requiresCharging = entry.getBoolean("requiresCharging"),
            requiresIdle = entry.getBoolean("requiresIdle"),
            conflictPolicy = decodeConflictPolicy(entry.getInt("conflictPolicy")) ?: continue,
        )
        val record = BackgroundTaskRecord(
            descriptor = descriptor,
            nextRegularRunUnixNs =
                if (entry.isNull("nextRegularRunUnixNs")) null else entry.getLong(
                    "nextRegularRunUnixNs",
                ),
        )

        records[descriptor.identifier] = record
    }

    return records
}

/**
 * Persist every durable background record into shared preferences.
 */
private fun storeRecords(
    preferences: SharedPreferences,
    records: Collection<BackgroundTaskRecord>,
) {
    val payload = JSONArray()

    for (record in records.sortedBy { value -> value.descriptor.identifier }) {
        val descriptor = record.descriptor

        payload.put(
            JSONObject()
                .put("identifier", descriptor.identifier)
                .put("trigger", encodeTrigger(descriptor.trigger))
                .put("scheduleKind", encodeScheduleKind(descriptor.schedule.kind))
                .put("earliestBeginUnixNs", descriptor.schedule.earliestBeginUnixNs)
                .put("repeatIntervalNs", descriptor.schedule.repeatIntervalNs)
                .put("network", encodeNetwork(descriptor.network))
                .put("requiresCharging", descriptor.requiresCharging)
                .put("requiresIdle", descriptor.requiresIdle)
                .put("conflictPolicy", encodeConflictPolicy(descriptor.conflictPolicy))
                .put("nextRegularRunUnixNs", record.nextRegularRunUnixNs),
        )
    }

    preferences.edit()
        .putString(backgroundRecordsKey, payload.toString())
        .apply()
}

/**
 * Encode one runtime trigger kind as one stored integer tag.
 */
private fun encodeTrigger(
    trigger: RuntimeHostBackgroundTriggerKind,
): Int {
    return when (trigger) {
        RuntimeHostBackgroundTriggerKind.AppRefresh -> 1
        RuntimeHostBackgroundTriggerKind.Processing -> 2
    }
}

/**
 * Decode one stored integer tag as one runtime trigger kind.
 */
private fun decodeTrigger(
    value: Int,
): RuntimeHostBackgroundTriggerKind? {
    return when (value) {
        1 -> RuntimeHostBackgroundTriggerKind.AppRefresh
        2 -> RuntimeHostBackgroundTriggerKind.Processing
        else -> null
    }
}

/**
 * Encode one runtime network requirement as one stored integer tag.
 */
private fun encodeNetwork(
    network: RuntimeHostBackgroundNetworkRequirement,
): Int {
    return when (network) {
        RuntimeHostBackgroundNetworkRequirement.None -> 1
        RuntimeHostBackgroundNetworkRequirement.Connected -> 2
        RuntimeHostBackgroundNetworkRequirement.Unmetered -> 3
    }
}

/**
 * Encode one runtime schedule kind as one stored integer tag.
 */
private fun encodeScheduleKind(
    kind: RuntimeHostBackgroundTaskScheduleKind,
): Int {
    return when (kind) {
        RuntimeHostBackgroundTaskScheduleKind.Once -> 1
        RuntimeHostBackgroundTaskScheduleKind.Recurring -> 2
    }
}

/**
 * Decode one stored integer tag as one runtime schedule kind.
 */
private fun decodeScheduleKind(
    value: Int,
): RuntimeHostBackgroundTaskScheduleKind? {
    return when (value) {
        1 -> RuntimeHostBackgroundTaskScheduleKind.Once
        2 -> RuntimeHostBackgroundTaskScheduleKind.Recurring
        else -> null
    }
}

/**
 * Decode one stored integer tag as one runtime network requirement.
 */
private fun decodeNetwork(
    value: Int,
): RuntimeHostBackgroundNetworkRequirement? {
    return when (value) {
        1 -> RuntimeHostBackgroundNetworkRequirement.None
        2 -> RuntimeHostBackgroundNetworkRequirement.Connected
        3 -> RuntimeHostBackgroundNetworkRequirement.Unmetered
        else -> null
    }
}

/**
 * Encode one runtime conflict policy as one stored integer tag.
 */
private fun encodeConflictPolicy(
    policy: RuntimeHostBackgroundConflictPolicy,
): Int {
    return when (policy) {
        RuntimeHostBackgroundConflictPolicy.Replace -> 1
        RuntimeHostBackgroundConflictPolicy.Keep -> 2
    }
}

/**
 * Decode one stored integer tag as one runtime conflict policy.
 */
private fun decodeConflictPolicy(
    value: Int,
): RuntimeHostBackgroundConflictPolicy? {
    return when (value) {
        1 -> RuntimeHostBackgroundConflictPolicy.Replace
        2 -> RuntimeHostBackgroundConflictPolicy.Keep
        else -> null
    }
}
