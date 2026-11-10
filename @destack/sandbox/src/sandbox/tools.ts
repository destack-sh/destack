import { dirname, join } from "node:path";
import type { OperatingSystem, Process } from "@destack/environment";

/** The tools the Linux sandbox runs beside bubblewrap, ripgrep and socat. */
export const SANDBOX_TOOLS = ["ripgrep", "socat"] as const;

/** The file name a Linux release ships each tool under, beside its launcher. */
const FILES: Readonly<Record<SandboxTool, string>> = { ripgrep: "rg", socat: "socat" };

/** A tool the Linux sandbox runs: ripgrep finds denied files, socat reaches the proxy. */
export type SandboxTool = (typeof SANDBOX_TOOLS)[number];

/** A tool the Linux sandbox runs beside bubblewrap. */
export const SandboxTool = {
    /** Read the file name a Linux release ships a tool under. */
    file(tool: SandboxTool): string {
        return FILES[tool];
    },

    /** Find the tool a Linux release ships beside its programs, none for a checkout. */
    path(
        tool: SandboxTool,
        process: Pick<Process, "runtime" | "executable">,
        os: OperatingSystem,
    ): string | undefined {
        return os === "linux" && process.runtime.isStandalone
            ? join(dirname(process.executable), FILES[tool])
            : undefined;
    },
};
