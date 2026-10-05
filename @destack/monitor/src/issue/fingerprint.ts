import type { StackFrame } from "./frame.ts";
import type { Exception } from "./exception.ts";

/** The fingerprint part standing for the default grouping, after Sentry. */
const DEFAULT_PART = "{{ default }}";

/** Group an exception: by its error type and the symbols of its in-app frames, else its message, unless its capture names the grouping. */
export async function fingerprint(
    exception: Pick<Exception, "errorType" | "message" | "fingerprint">,
    frames: readonly StackFrame[],
): Promise<string> {
    // group by the in-app frames' symbols, else their source locations, keeping one of each recursion
    const parts = frames
        .filter((frame) => frame.isInApp)
        .map((frame) => frame.moniker ?? `${frame.path ?? frame.file}:${frame.function ?? ""}`)
        .filter((part, index, all) => part !== all[index - 1]);

    // group a failure without in-app frames by its message
    const grouping = [exception.errorType, ...(parts.length === 0 ? [exception.message] : parts)];

    // replace the grouping by the capture's, expanding its default part
    const requested = exception.fingerprint?.flatMap((part) =>
        part === DEFAULT_PART ? grouping : [part],
    );

    return digest(requested ?? grouping);
}

/** Digest grouping parts as SHA-256 hexadecimal digits. */
async function digest(parts: readonly string[]): Promise<string> {
    const bytes = new TextEncoder().encode(JSON.stringify(parts));

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}
