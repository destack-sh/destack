import type { AuditDestination } from "@destack/audit/server";
import type { DatabaseConnection } from "@destack/db";
import type { Directory } from "@destack/directory";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { Duration, Identifier } from "@destack/schema";
import type { CallKey } from "@destack/service/request";
import type { ServiceImplementation } from "@destack/service/server";
import { binding, capture, deployment, installation, space } from "@destack/space/object";
import type { Uplink } from "@destack/sync";
import type { Keyring } from "@destack/identity";
import { KeyringVaultHost } from "../key/index.ts";
import { vaultProvider } from "../provider/index.ts";
import { vaultService } from "../service/index.ts";
import { serveSecrets, vault } from "./secret.ts";

/** What a cell serves its spaces' vaults with: their database, the host keeping their contents and the spaces it follows. */
export interface VaultOptions {
    /** The database keeping the vaults and their secrets, with copies of the spaces and their installations, bindings and captures. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The directory keeping the claims of resource names across kinds. */
    readonly directory: Directory;
    /** The keyring encrypting the vaults' keys. */
    readonly keyring: Keyring;
    /** The host the vaults' keys belong to, which their encryption context names. */
    readonly location: string;
    /** The machine keeping the vaults it provisions, absent for a region. */
    readonly machine: Identifier<"machine"> | null;
    /** The cell serving the spaces, which with the service's name names the copy of their rows. */
    readonly cell: string;
    /** The space service's uplink, streaming the spaces' rows, installations, bindings and captures and receiving their changes. */
    readonly spaces: Uplink;
    /** How long deleted secrets stay restorable, 30 days by default. */
    readonly recovery?: Duration;
    /** The audit history the journal delivers calls to. */
    readonly history?: AuditDestination;
}

/** The vault service with the object server keeping the vaults. */
export interface VaultImplementation extends ServiceImplementation {
    /** The object server keeping the vaults and their secrets, home of the space service's copies. */
    readonly objects: ObjectServer;
}

/** Implement the vault service: serve a cell's vaults, secrets and versions, provisioning and retiring the vaults through a host. */
export function implementVault(options: VaultOptions): VaultImplementation {
    // serve the vaults and their secrets over the followed spaces
    const vaults = new KeyringVaultHost(options.database, options.keyring, options.location);
    const { secret, version } = serveSecrets(vaults, options.recovery);
    const objects: ObjectServer = new ObjectServer({
        objects: { vault, secret, version },
        policies: [space, installation, binding, capture, deployment],
        database: options.database,
        callKey: options.callKey,
        directory: options.directory,
        origin: { package: vaultService.package, service: vaultService.name },
        ...(options.history === undefined ? {} : { history: options.history }),
        subscriber: Subscriber.of(options.spaces, () =>
            objects.source.workloadSubscriptions(`${options.cell}/${vaultService.name}`),
        ),
        provisioned: { providers: [vaultProvider(vaults)], machine: options.machine },
    });

    return { ...objects.implement(vaultService), objects };
}
