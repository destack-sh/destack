import type { Environment } from "@destack/environment";
import type { TelemetryScope } from "@destack/telemetry";

/** A string kept under a key in a page's storage, as solid-primitives' `makePersisted` keeps it. */
export class StoredItem {
    /** The key the string is kept under. */
    readonly key: string;
    /** The page's environment, whose storage keeps the string. */
    readonly #environment: Environment<"storage">;
    /** The telemetry that hears of storage refusing a read or write. */
    readonly #telemetry: TelemetryScope;

    /** Keep a string under a key in the storage of a page's environment. */
    constructor(key: string, environment: Environment<"storage">, telemetry: TelemetryScope) {
        this.key = key;
        this.#environment = environment;
        this.#telemetry = telemetry;
    }

    /** Read the string, undefined where storage keeps none or the browser refuses it. */
    read(): string | undefined {
        // read the string, reporting the browser's refusal and failing on anything else
        try {
            return this.#environment.storage.getItem(this.key) ?? undefined;
        } catch (error) {
            this.#report(error);

            return undefined;
        }
    }

    /** Write the string, reporting storage that refuses it. */
    write(value: string): void {
        try {
            this.#environment.storage.setItem(this.key, value);
        } catch (error) {
            this.#report(error);
        }
    }

    /** Report storage the browser blocks or fills up, rethrowing every other failure. */
    #report(error: unknown): void {
        if (!(error instanceof DOMException)) {
            throw error;
        }
        this.#telemetry.captureException(error, { level: "warning" });
    }
}
