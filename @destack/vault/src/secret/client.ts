import type { Capture } from "@destack/space/object";
import { identifier, type Identifier } from "@destack/schema";
import { createClient, type ClientOptions } from "@destack/service/client";
import { vaultService } from "../service/index.ts";

/** A secret version a deployment captured, read through the vault. */
export class Secret {
    /** The vault client, authenticated as the workload's installation. */
    readonly client: ReturnType<typeof connect>;
    /** The space of the secret. */
    readonly spaceId: Capture["scope"];
    /** The secret the deployment's binding targets. */
    readonly secretId: Identifier<"secret">;
    /** The version the deployment captured for its workloads. */
    readonly version: number;

    /** Take the secret a deployment captured for one of its declarations. */
    constructor(
        client: ReturnType<typeof connect>,
        captured: Pick<Capture, "scope" | "target" | "version">,
    ) {
        // keep the client and the captured secret version
        this.client = client;
        this.spaceId = captured.scope;
        this.secretId = identifier("secret").parse(captured.target);
        this.version = captured.version;
    }

    /** Read the captured version's value. */
    read() {
        return this.client.secret.read({
            spaceId: this.spaceId,
            id: this.secretId,
            version: this.version,
        });
    }
}

/** Connect to an authenticated local or regional vault endpoint. */
export function connect(options: ClientOptions) {
    return createClient(vaultService, options);
}
