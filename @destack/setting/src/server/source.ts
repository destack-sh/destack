import { eq } from "@destack/db";
import type { PackageId } from "@destack/package";
import { RecordProvenance } from "@destack/model/source";
import type { ServiceContext } from "@destack/service/server";
import {
    SettingStore,
    reconcileSettings,
    decodeAssignment,
    decodePolicy,
    type SettingReconciliation,
} from "../database/index.ts";
import { sourceKey } from "../database/source.ts";
import { settingAssignment, settingPolicy } from "../stack/index.ts";
import type { SettingServerOptions } from "./server.ts";
import { reportSettingError } from "./error.ts";

/** Source application requires permission to apply the selected immutable revision. */
export interface SettingSourceOptions extends SettingServerOptions {
    /** Verify the source owner, revision and selected package before applying declarations. */
    readonly authorizeSource: (
        context: ServiceContext,
        source: SettingReconciliation["source"],
        packageId: PackageId,
    ) => Promise<void>;
}

/** Apply source settings after authorizing new values and every previously managed selection. */
export async function applySettings(
    store: SettingStore,
    input: SettingReconciliation,
    packageId: PackageId,
    options: SettingSourceOptions,
    context: ServiceContext,
) {
    // verify the source independently of the permissions its declarations request
    const caller = context.requireCaller();
    await options.authorizeSource(context, input.source, packageId);
    const installationId =
        input.source.kind === "package" ? input.source.installationId : undefined;
    const declarations = await options.declarations(context, packageId, installationId);
    const audit = await options.audit(context);
    const source = sourceKey(RecordProvenance.parse({ ...input.source, name: "settings" }));

    // apply the source records in one transaction
    const result = await store.database
        .transaction(
            async (database) => {
                // authorize retained selections as well as new ones so removal cannot bypass access checks
                if (input.assignments !== undefined) {
                    const previous = await database
                        .select()
                        .from(settingAssignment)
                        .where(eq(settingAssignment.source, source));
                    for (const record of previous) {
                        await options.authorize(
                            context,
                            decodeAssignment(record).target,
                            "write",
                            packageId,
                        );
                    }
                    for (const definition of Object.values(input.assignments)) {
                        await options.authorize(context, definition.target, "write", packageId);
                    }
                }

                // require administrative access before recommendations or required values change
                if (input.policies !== undefined) {
                    const previous = await database
                        .select()
                        .from(settingPolicy)
                        .where(eq(settingPolicy.source, source));
                    for (const record of previous) {
                        await options.authorizePolicy(
                            context,
                            decodePolicy(record).authority,
                            "write",
                        );
                    }
                    for (const definition of Object.values(input.policies)) {
                        await options.authorizePolicy(context, definition.authority, "write");
                    }
                }

                // commit source revision, assignments, policies and audit records together

                return reconcileSettings(new SettingStore(database), input, declarations, {
                    subject: caller.authentication.subject,
                    audit,
                });
            },
            { isolationLevel: "serializable" },
        )
        .catch(reportSettingError);
    store.notify();

    return result;
}
