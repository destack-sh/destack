package dev.destack.runtime.android.core

/**
 * The primary renderer surface family for one Android host embedder.
 */
public enum class RendererSurfaceKind {
    /**
     * One `SurfaceView` backed host surface.
     */
    SurfaceView,

    /**
     * One `TextureView` backed host surface.
     */
    TextureView,

    /**
     * One `SurfaceControlViewHost` or similar embedded host surface.
     */
    EmbeddedHostSurface,
}

/**
 * The primary renderer surface for one Android host embedder.
 */
public data class RendererSurface(
    /**
     * The primary renderer surface kind.
     */
    val kind: RendererSurfaceKind,

    /**
     * The stable logical identifier for this host surface.
     */
    val identifier: String,
)
