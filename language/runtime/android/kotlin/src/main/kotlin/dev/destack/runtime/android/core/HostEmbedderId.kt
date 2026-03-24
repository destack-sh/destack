package dev.destack.runtime.android.core

/**
 * The stable host embedder identifier for one attached runtime session.
 */
@JvmInline
public value class HostEmbedderId(
    /**
     * The raw embedder identifier value.
     */
    public val rawValue: Long,
)
