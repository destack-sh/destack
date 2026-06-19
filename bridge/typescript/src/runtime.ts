/** Return whether this runtime exposes Node process metadata. */
export function hasNodeProcess(): boolean {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;

    return processValue?.versions?.node != null;
}
