import type { Service } from "@destack/service";
import type { ClientOptions } from "@destack/service/client";
import { secret, secretVersion, vault } from "../object/index.ts";

/** A client of the vault service's secrets and versions. */
export class VaultClient {
    /** The vault methods. */
    readonly vault;
    /** The secret methods. */
    readonly secret;
    /** The version methods. */
    readonly version;

    /** Connect to the secrets and versions of a service, as the options authenticate. */
    constructor(service: Pick<Service, "package">, options: ClientOptions) {
        this.vault = vault.connect(service, options);
        this.secret = secret.connect(service, options);
        this.version = secretVersion.connect(service, options);
    }
}
