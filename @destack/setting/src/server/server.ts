import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { ObjectServer } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { Identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
import { settingService } from "../service/index.ts";
import { settingJournal } from "../stack/index.ts";
import { servedObjects } from "./value.ts";

/** The database, releases and audit a setting host serves with. */
export interface SettingServiceOptions {
    /** The database holding the values. */
    readonly database: DatabaseConnection;
    /** Open the build of a package's release, the installation's when given. */
    readonly release: (
        packageId: PackageId,
        installation?: Identifier<"installation">,
    ) => Promise<BuildReader>;
    /** Open the audit recorder of a scope. */
    readonly audit: (scope: string, context?: ServiceContext) => AuditRecorder<DatabaseConnection>;
}

/** Serve setting values as objects. */
export function implementService(options: SettingServiceOptions): ServiceImplementation {
    const objects = new ObjectServer({
        objects: servedObjects(options.release),
        database: options.database,
        journal: new Journal(settingJournal),
        audit: options.audit,
    });

    return {
        ...objects.implement(settingService),
        responseHeaders: { "Cache-Control": "no-store" },
    };
}
