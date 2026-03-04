package com.symbol.destack;

import com.sun.jna.Library;
import com.sun.jna.Native;

/**
 * Native capi function bindings.
 */
final class CapiBindings {
    /**
     * The exported capi function surface.
     */
    interface CapiLibrary extends Library {
        int destack_capi_abi_version();
        String destack_capi_version();
        byte destack_capi_is_available();
    }

    private static final CapiLibrary LIBRARY = loadLibrary();

    private CapiBindings() {}

    static int abiVersion() {
        if (LIBRARY == null) {
            return 0;
        }

        return LIBRARY.destack_capi_abi_version();
    }

    static String version() {
        if (LIBRARY == null) {
            return Destack.VERSION;
        }

        String version = LIBRARY.destack_capi_version();
        if (version == null || version.isBlank()) {
            return Destack.VERSION;
        }

        return version;
    }

    static boolean isAvailable() {
        if (LIBRARY == null) {
            return false;
        }

        return LIBRARY.destack_capi_is_available() != 0;
    }

    // load the capi library from explicit path or default names
    private static CapiLibrary loadLibrary() {
        String explicitPath = System.getenv("DESTACK_CAPI_LIB");
        if (explicitPath != null && !explicitPath.isBlank()) {
            try {
                return Native.load(explicitPath, CapiLibrary.class);
            } catch (UnsatisfiedLinkError ignored) {
                return null;
            }
        }

        String[] candidates = {"destack_capi", "libdestack_capi"};
        for (String candidate : candidates) {
            try {
                return Native.load(candidate, CapiLibrary.class);
            } catch (UnsatisfiedLinkError ignored) {
                // keep searching
            }
        }

        return null;
    }
}
