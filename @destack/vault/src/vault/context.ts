import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import type { Secret } from "../secret/index.ts";
import type { vaultService } from "../service/index.ts";
import type { Caller } from "@destack/service/authentication";
import type { PackageId } from "@destack/package";

/** Exact operation names derived from the public service router. */
export type VaultOperation = {
    [
        Domain in keyof typeof vaultService.router
    ]: `${Domain}.${keyof (typeof vaultService.router)[Domain] & string}`;
}[keyof typeof vaultService.router];

/** Authenticated caller and durable audit recorder established by the receiving host. */
export interface VaultContext {
    /** Receiving package selected by the host. */
    audience: PackageId;
    /** Shared, audience-qualified authentication result. */
    caller: Caller;
    /** Durable recorder bound to the verified caller and this database. */
    audit: AuditRecorder<DatabaseConnection>;
}
/** The persisted target checked against regional grants and deployment bindings. */
export interface VaultAccess {
    /** Exact operation, such as version.read or secret.create. */
    operation: VaultOperation;
    /** Selected space. */
    spaceId: Secret["spaceId"];
    /** Containing vault, derived from persisted metadata for secret operations. */
    vaultId?: Secret["vaultId"];
    /** Secret identity, absent for collection operations. */
    secretId?: Secret["id"];
    /** Exact version when applicable. */
    version?: number;
}
