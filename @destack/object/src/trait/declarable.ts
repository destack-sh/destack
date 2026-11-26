import { AccessError, Manager } from "@destack/access";
import { t } from "@destack/locale";
import { MethodBuilder } from "../method/builder.ts";
import type { ManagedTable } from "./record.ts";
import { callableTrait } from "./trait.ts";

/** The method declarations of records a stack declaration can manage. */
const managerMethod = new MethodBuilder<ManagedTable>();

/** Let stacks declare the objects, filling the listed create input fields. */
export const declarable = callableTrait("declarable", {});

/** The methods of records a stack declaration manages: detaching them and attaching them again. */
export const Declarable = {
    /** Stop the declaration managing a record, which callers may then change. */
    detach: managerMethod
        .mutation({ title: t.context("action")`Detach`, permission: "manage" })
        .handle(async (context) => {
            // refuse a record no declaration manages, and leave a detached one as it is
            const { target } = context;
            if (target.managerInstallationId === null) {
                throw AccessError.of(
                    "DECLARATION_BLOCKED",
                    `cannot detach ${context.object.name} '${target.id}': it has no declaration`,
                );
            } else if (!Manager.isManaged(target)) {
                return target;
            }

            return context.updateStatus({ detachedAt: context.now });
        }),

    /** Hand a detached record back to its declaration, which takes it over on its next apply. */
    attach: managerMethod
        .mutation({ title: t.context("action")`Attach`, permission: "manage" })
        .handle(async (context) => {
            // refuse a record no declaration manages, and leave an attached one as it is
            const { target } = context;
            if (target.managerInstallationId === null) {
                throw AccessError.of(
                    "DECLARATION_BLOCKED",
                    `cannot attach ${context.object.name} '${target.id}': it has no declaration`,
                );
            } else if (Manager.isManaged(target)) {
                return target;
            }

            return context.updateStatus({ detachedAt: null });
        }),
};
