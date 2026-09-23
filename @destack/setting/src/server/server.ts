import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
import { createProcedureAudit } from "@destack/audit/server";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import type { PackageId } from "@destack/package";
import type { schema } from "@destack/schema";
import type {
    Setting,
    SettingReference,
    SettingSelection,
    SettingAuthority,
    SettingPolicy,
    SettingLocation,
} from "../setting/index.ts";
import type { SettingStore } from "../database/index.ts";
import { settingRouter } from "./setting.ts";
import { assignmentRouter } from "./assignment.ts";
import { policyRouter } from "./policy.ts";

/** Host bindings required to resolve and edit settings under verified identity. */
export interface SettingServerOptions {
    /** End subscriptions when the hosting service begins shutdown. */
    readonly signal?: AbortSignal;
    /** Verify the issuing account, space or host before policy administration. */
    readonly authorizePolicy: (
        context: ServiceContext,
        authority: SettingAuthority,
        operation: "read" | "write",
    ) => Promise<void>;
    /** Resolve declarations from the host-selected consumer release. */
    readonly declarations: (
        context: ServiceContext,
        packageId: PackageId,
        installationId?: schema.Infer<typeof SettingLocation>["installationId"],
    ) => Promise<readonly Setting[]>;
    /** Authorize exact targets, consuming packages and operations before reading storage. */
    readonly authorize: (
        context: ServiceContext,
        target: SettingSelection,
        operation: "read" | "write",
        packageId?: PackageId,
    ) => Promise<void>;
    /** Read all applicable policy under its authoritative freshness interval. */
    readonly policies: (
        context: ServiceContext,
        settings: readonly SettingReference[],
        target: SettingSelection,
        database: DatabaseConnection,
    ) => Promise<{ policies: readonly SettingPolicy[]; validUntil: number | null }>;
    /** Attribute audit events to the verified caller and host. */
    readonly audit: (context: ServiceContext) => Promise<AuditRecorder<DatabaseConnection>>;
}

/** Connect authenticated settings procedures to persistence and host authorization. */
export function implementService(
    store: SettingStore,
    options: SettingServerOptions,
): ServiceImplementation {
    return {
        authorize: async ({ context }) => {
            context.requireCaller();
        },
        audit: createProcedureAudit(({ context }) => options.audit(context)),
        responseHeaders: { "Cache-Control": "no-store" },
        router: {
            setting: settingRouter(store, options),
            assignment: assignmentRouter(store, options),
            policy: policyRouter(store, options),
        },
    };
}
