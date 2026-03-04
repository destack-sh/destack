package industries.symbol.destack;

/**
 * A client type for Destack.
 */
public final class Client {
    /**
     * Return the backend marker.
     */
    public String backend() {
        return Destack.BACKEND;
    }

    /**
     * Return the loaded C ABI version.
     */
    public int capiAbiVersion() {
        return CapiBindings.abiVersion();
    }

    /**
     * Return whether the C API reports availability.
     */
    public boolean capiIsAvailable() {
        return CapiBindings.isAvailable();
    }

    /**
     * Return the package version.
     */
    public String version() {
        return CapiBindings.version();
    }
}
