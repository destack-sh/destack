import { implement, type ServiceContext } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { settingService } from "../service/index.ts";
import {
    getAssignment,
    listAssignment,
    mutateAssignment,
    type SettingStore,
} from "../database/index.ts";
import type { SettingServerOptions } from "./server.ts";
import { declaration } from "./setting.ts";
import { reportSettingError } from "./error.ts";

/** Connect exact-target assignment reads and mutations to authorization and persistence. */
export function assignmentRouter(store: SettingStore, options: SettingServerOptions) {
    const implementation = implement(settingService.router).$context<ServiceContext>();

    return {
        detach: implementation.assignment.detach.handler(async ({ input, context }) => {
            await options.authorize(context, input.target, "write", input.packageId);
            const setting = await declaration(
                input.setting,
                context,
                options,
                input.packageId,
                input.target,
            );

            return mutateAssignment(
                store,
                setting,
                { ...input, expectedRevision: input.revision },
                {
                    subject: context.requireCaller().authentication.subject,
                    audit: await options.audit(context),
                },
                "detach",
            ).catch(reportSettingError);
        }),
        get: implementation.assignment.get.handler(async ({ input, context }) => {
            // read the assignment and report whether the caller may edit it
            await options.authorize(context, input.target, "read");
            const assignment = await getAssignment(store.database, input.setting, input.target);
            let edit: "allowed" | "denied" | "source" =
                assignment?.provenance && assignment.detachedAt === null ? "source" : "allowed";
            try {
                await options.authorize(context, input.target, "write");
            } catch (error) {
                if (!(error instanceof ServiceError) || error.code !== "FORBIDDEN") {
                    throw error;
                }
                edit = "denied";
            }

            return {
                assignment,
                edit,
            };
        }),
        list: implementation.assignment.list.handler(async ({ input, context }) => {
            await options.authorize(context, input.target, "read");

            return listAssignment(
                store.database,
                input.target,
                input.limit,
                input.cursor,
                input.packageId,
            );
        }),
        set: implementation.assignment.set.handler(async ({ input, context }) => {
            await options.authorize(context, input.target, "write", input.packageId);
            const setting = await declaration(
                input.setting,
                context,
                options,
                input.packageId,
                input.target,
            );

            return mutateAssignment(store, setting, input, {
                subject: context.requireCaller().authentication.subject,
                audit: await options.audit(context),
            }).catch(reportSettingError);
        }),
        reset: implementation.assignment.reset.handler(async ({ input, context }) => {
            await options.authorize(context, input.target, "write", input.packageId);
            const setting = await declaration(
                input.setting,
                context,
                options,
                input.packageId,
                input.target,
            );

            return mutateAssignment(
                store,
                setting,
                input,
                {
                    subject: context.requireCaller().authentication.subject,
                    audit: await options.audit(context),
                },
                "reset",
            ).catch(reportSettingError);
        }),
    };
}
