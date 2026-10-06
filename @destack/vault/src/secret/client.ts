import type { Capture } from "@destack/space/object";
import { schema, type Identifier } from "@destack/schema";
import type { SecretClient } from "../object/index.ts";

/** A secret version a deployment captured, read through the vault. */
export class Secret {
    /** The vault client, authenticated as the workload's installation. */
    readonly client: SecretClient;
    /** The space of the secret. */
    readonly spaceId: Capture["scope"];
    /** The secret the deployment's binding targets. */
    readonly secretId: Identifier<"secret">;
    /** The version the deployment captured for its workloads. */
    readonly version: number;

    /** Take the secret a deployment captured for one of its declarations. */
    constructor(
        client: SecretClient,
        captured: Pick<Capture, "scope" | "target"> & { readonly version: number },
    ) {
        // keep the client and the captured secret version
        this.client = client;
        this.spaceId = captured.scope;
        this.secretId = schema.identifier("secret").parse(captured.target);
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
