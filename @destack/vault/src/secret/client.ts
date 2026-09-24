import { createClient, type ClientOptions } from "@destack/service/client";
import { vaultService } from "../service/index.ts";
import type { SecretReference } from "../declare/index.ts";

/** Select one installation-bound secret through an authenticated vault client. */
export class BoundSecret {
    /** Typed authenticated service client. */
    readonly client: ReturnType<typeof connect>;
    /** Exact selection supplied by the installation's trusted runtime. */
    readonly reference: SecretReference;
    /** Bind an already authorized secret selection. */
    constructor(client: ReturnType<typeof connect>, reference: SecretReference) {
        this.client = client;
        this.reference = Object.freeze({ ...reference });
    }
    /** Read the selected version under current server authority. */
    read() {
        return this.client.version.read({
            spaceId: this.reference.space,
            secretId: this.reference.secret,
            version: this.reference.version,
        });
    }
}

/** Connect to an authenticated local or regional vault endpoint. */
export function connect(options: ClientOptions) {
    return createClient(vaultService.router, options);
}
