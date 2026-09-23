/** An explicit setting resolution failure without private assignment contents. */
export class SettingError extends Error {
    /** Machine-readable failure category. */
    readonly code: "INVALID_TARGET" | "INVALID_VALUE" | "CONFLICT" | "STALE_POLICY";

    /** Report a declaration or resolution failure. */
    constructor(code: SettingError["code"], message: string) {
        super(message);
        this.name = "SettingError";
        this.code = code;
    }
}
