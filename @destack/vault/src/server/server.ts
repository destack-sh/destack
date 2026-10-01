import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import { Duration } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { Journal, type JournalKey } from "@destack/service/database";
import type { ServiceImplementation } from "@destack/service/server";
import { space } from "@destack/space/object";
import { vault } from "../object/index.ts";
import { vaultService } from "../service/index.ts";
import { vaultJournal } from "../stack/index.ts";
import { servedObjects } from "./secret.ts";
import type { Keyring } from "../encryption/index.ts";

/** The shortest recovery window a host may give deleted secrets: one day. */
const MINIMUM_RECOVERY: Duration = { days: 1 };

/** The database, root keys and recovery window a vault host serves with. */
export interface VaultServiceOptions {
    /** The space database with vaults, secrets and their values. */
    readonly database: DatabaseConnection;
    /** Read the deployment's key that sensitive inputs are fingerprinted under. */
    readonly journalKey: JournalKey;
    /** The host's root keys, wrapping its vaults' keys. */
    readonly keyring: Keyring;
    /** The host the values are stored at, which every ciphertext authenticates. */
    readonly location: string;
    /** How long deleted secrets stay restorable, one day at least. */
    readonly recovery: Duration;
}

/** Serve vaults, secrets and versions as objects. */
export function implementService(options: VaultServiceOptions): ServiceImplementation {
    // require a recovery window of a day at least
    if (Duration.milliseconds(options.recovery) < Duration.milliseconds(MINIMUM_RECOVERY)) {
        throw new TypeError("vault recovery window is shorter than a day");
    }

    // serve the objects over the vault database, deciding in each space through its roles
    const audit = AuditRecorder.service(new AuditOutbox(options.database), {
        package: vault.package,
        service: "vault",
    });
    const objects = new ObjectServer({
        objects: servedObjects(options.keyring, options.location, options.recovery),
        policies: [space],
        database: options.database,
        journal: new Journal(vaultJournal, options.journalKey),
        audit,
    });

    return {
        ...objects.implement(vaultService),
        responseHeaders: { "Cache-Control": "no-store", Pragma: "no-cache" },
    };
}
