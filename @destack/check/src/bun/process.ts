/// <reference types="bun" />
/** The Bun process running this module. */
export const BunProcess = {
    /** Whether this process is a standalone Bun executable. */
    isStandalone: "Bun" in globalThis && Bun.isStandaloneExecutable,

    /** Read the Git revision of the Bun build running this process. */
    revision(): string {
        return Bun.revision;
    },
};
