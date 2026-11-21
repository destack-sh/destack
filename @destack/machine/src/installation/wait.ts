import { setting } from "@destack/setting/object";
import { installation, InstallationWait } from "@destack/stack/object";
import type { ObjectType } from "@destack/object";
import type { PlanWait } from "@destack/object/server";
import { tag } from "@destack/registry/object";
import type { ApplicationOptions } from "./application.ts";
import { StackError } from "@destack/stack/error";

/** List the object types an installation may wait for, which its controller watches. */
export function waitedTypes(options: Pick<ApplicationOptions, "binder" | "space">): ObjectType[] {
    // take settings, installations and bound types, and tags where the machine builds
    const followed = options.space.machine.isBuilding ? [tag] : [];
    const waited = [setting, installation, ...options.binder.targets(), ...followed];

    return waited.filter(
        (object, index) => waited.findIndex((each) => each.same(object)) === index,
    );
}

/** Record the row a plan waits for as its installation's wait. */
export function planWait(
    wait: PlanWait,
    options: Pick<ApplicationOptions, "binder" | "space">,
): InstallationWait {
    // refuse a type the controller does not watch here, which would never wake the installation
    if (!waitedTypes(options).some((object) => object.same(wait.object))) {
        throw StackError.of(
            "DEPENDENCY_BLOCKED",
            `cannot apply the stack: it waits for a ${wait.object.name}, which this machine does not watch`,
        );
    }

    return InstallationWait.of(wait.object, wait.condition);
}
