import { AccessError, Manager } from "@destack/access";
import { MethodBuilder } from "./record.ts";
import type { ManagedTable } from "../trait/record.ts";

/** The method declarations of records a stack declaration can manage. */
const managerMethod = new MethodBuilder<ManagedTable>();

/** Stop the declaration managing a record, which callers may then change. */
export const detach = managerMethod.mutation({ permission: "manage" }).handle(async (context) => {
    // refuse a record no declaration manages, and leave a detached one as it is
    const { target } = context;
    if (target.managerInstallationId === null) {
        throw AccessError.of("UNMANAGED", `${context.object.name} ${target.id} has no declaration`);
    } else if (!Manager.isManaged(target)) {
        return target;
    }

    return context.updateStatus({ detachedAt: context.now });
});

/** Hand a detached record back to its declaration, which takes it over on its next apply. */
export const attach = managerMethod.mutation({ permission: "manage" }).handle(async (context) => {
    // refuse a record no declaration manages, and leave an attached one as it is
    const { target } = context;
    if (target.managerInstallationId === null) {
        throw AccessError.of("UNMANAGED", `${context.object.name} ${target.id} has no declaration`);
    } else if (Manager.isManaged(target)) {
        return target;
    }

    return context.updateStatus({ detachedAt: null });
});
