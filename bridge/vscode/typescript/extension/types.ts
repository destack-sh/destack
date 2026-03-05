/**
 * The language client state labels exposed to host tests.
 */
export type DestackClientState = "starting" | "running" | "stopped";

/**
 * The runtime state payload exposed by the extension testing API.
 */
export type DestackRuntimeState = {
    /**
     * The current language client lifecycle state.
     */
    clientState: DestackClientState;
    /**
     * Whether a configuration-triggered restart is currently running.
     */
    isConfigurationRestartInFlight: boolean;
    /**
     * The total number of completed restart attempts.
     */
    restartCount: number;
    /**
     * The active server process identifier.
     */
    serverProcessId: number | null;
};
