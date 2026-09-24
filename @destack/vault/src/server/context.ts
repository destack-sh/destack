import { createRecorder } from "@destack/audit/server";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import type { ServiceContext } from "@destack/service/server";
import type { space } from "@destack/model/regional";
import { vaultPackage } from "../audit/index.ts";

/** Select the vault event origin and durable writer for shared request attribution. */
export function vaultAudit(
    request: ServiceContext,
    database: DatabaseConnection,
    selected?: Pick<typeof space.$inferSelect, "id" | "accountId">,
) {
    return createRecorder(request, new AuditOutbox(database), {
        package: vaultPackage,
        service: "vault",
        spaceId: selected?.id,
        accountId: selected?.accountId ?? undefined,
    });
}
