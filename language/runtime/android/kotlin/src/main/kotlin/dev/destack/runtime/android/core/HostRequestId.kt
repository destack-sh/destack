package dev.destack.runtime.android.core

/**
 * One stable interactive host request identifier.
 */
@JvmInline
public value class HostRequestId(
    /**
     * The raw request identifier value.
     */
    public val rawValue: Long,
)
