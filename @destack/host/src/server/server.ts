import { account, region } from "@destack/account/object";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import { ObjectServer } from "@destack/object/server";
import { Journal } from "@destack/service/database";
import type { ServiceImplementation } from "@destack/service/server";
import { host, hostKey } from "../object/index.ts";
import { hostService } from "../service/index.ts";
import { hostJournal } from "../stack/index.ts";

/** What the global tier serves hosts with. */
export interface HostServiceOptions {
    /** The global database holding hosts, their keys and the audit outbox its composer delivers. */
    readonly database: DatabaseConnection;
}

/** Serve hosts and their keys as objects, decided by their accounts' roles. */
export function implementService(options: HostServiceOptions): ServiceImplementation {
    // serve the objects over the global database, recording audit events in its outbox
    const audit = AuditRecorder.service(new AuditOutbox(options.database), {
        package: host.package,
        service: hostService.name,
    });
    const objects = new ObjectServer({
        objects: { host, hostKey },
        policies: [account.policy, region],
        database: options.database,
        audit,
        journal: new Journal(hostJournal),
    });

    return objects.implement(hostService);
}
