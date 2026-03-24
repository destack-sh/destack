package dev.destack.runtime.android.core

/**
 * The raw host ABI session handle for one attached runtime session.
 */
@JvmInline
public value class HostSessionHandle(
    /**
     * The raw session handle value.
     */
    public val rawValue: Long,
)
