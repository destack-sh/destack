import { Failure } from "@destack/error";
import { ObjectTypeReference } from "@destack/sync";
import { PackageError } from "@destack/package";
import { isContractProcedure } from "@orpc/contract";
import type { ProcedureRouter } from "../procedure/index.ts";
import { Address, Plan, type Comparator, type Step } from "@destack/resource";
import { PermissionReference, PermissionScope, type PermissionSet } from "@destack/access";
import { Version } from "@destack/schema";
import { PackageDescription } from "../declare/description.ts";
import type { ObjectDeclaration } from "../declare/package.ts";
import type { Package, PackageIdentity } from "../declare/package.ts";

/** The address of the oldest caller release a package serves in its release plans. */
const SINCE_TARGET = "since";

/** The address of a package's mounts in its release plans. */
const MOUNT_TARGET = "mount";

/** The address of the permissions a package asks of the space in its release plans. */
const ACCESS_TARGET = "access";

/** Describe a package: its objects, mounts and views. */
export function describePackage(handle: Package & PackageIdentity): PackageDescription {
    // refuse an object type whose members or traits it cannot serve
    const objects = Object.values(handle.objects);
    for (const object of objects) {
        try {
            object.requireValid();
        } catch (error) {
            const { message } = Failure.of(error);
            throw PackageError.of(
                "DEFINITION_INVALID",
                `object ${object.name} is invalid: ${message}`,
                {
                    cause: error,
                },
            );
        }
    }

    // refuse a mount answering below an object's routes, which would take its calls
    const roots = new Set<string | undefined>();
    const routers: ProcedureRouter[] = objects.map((object) => object.procedures);
    for (let router = routers.pop(); router !== undefined; router = routers.pop()) {
        // take a procedure's first path segment, and walk a nested router
        if (isContractProcedure(router)) {
            roots.add(router["~orpc"].route.path?.split("/")[1]);
        } else {
            routers.push(...Object.values(router));
        }
    }
    const shadowing = Object.values(handle.mounts)
        .map((mount) => mount.path)
        .find((path) => roots.has(path.split("/")[1]));
    if (shadowing !== undefined) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `mount path '${shadowing}' is invalid: it overlaps an object's routes`,
        );
    }

    return PackageDescription.parse({
        title: handle.title,
        description: handle.description,
        icon: handle.icon,
        color: handle.color,
        ...(handle.since === undefined ? {} : { since: handle.since }),
        objects: describeObjects(handle.id, handle.objects),
        protocols: handle.protocols.map((each) => each.id),
        mounts: Object.fromEntries(
            Object.values(handle.mounts).map((mount) => [mount.name, { path: mount.path }]),
        ),
        views: Object.keys(handle.views),
        follows: [
            ...new Map(
                handle.follows
                    .flatMap((shape) => shape.objects)
                    .map((type) => [
                        ObjectTypeReference.key(type.typeReference),
                        type.typeReference,
                    ]),
            ).values(),
        ],
        permissions: handle.permissions,
        capabilities: handle.capabilities,
        errors: handle.errors,
    });
}

/** Describe the object types a package routes, with hooks for its declared types only. */
export function describeObjects(
    packageId: string,
    objects: Readonly<Record<string, ObjectDeclaration>>,
): Record<
    string,
    {
        readonly type: ObjectDeclaration["typeReference"];
        readonly hooks: ObjectDeclaration["hooks"];
    }
> {
    return Object.fromEntries(
        Object.entries(objects).map(([route, routed]) => [
            route,
            {
                type: routed.typeReference,
                hooks: routed.typeReference.packageId === packageId ? routed.hooks : {},
            },
        ]),
    );
}

/** Plan a package's caller, mount and access changes between releases. */
export const comparePackage: Comparator = (earlier, later) => {
    // read both releases with the capabilities the installation enables at each
    const before = PackageDescription.parse(earlier.description);
    const after = PackageDescription.parse(later.description);
    const asked = PackageDescription.asks(before, earlier.capabilities, "space");
    const asking = PackageDescription.asks(after, later.capabilities, "space");

    return Plan.join([
        () => compareSince(before.since, after.since),
        () => compareMounts(before.mounts, after.mounts),
        () => compareAccess(asked, asking),
    ]);
};

/** Plan granting each permission a release asks of the space that the earlier one did not. */
function compareAccess(asked: PermissionSet, asking: PermissionSet): Plan {
    const steps = PermissionScope.options.flatMap((scope): Step[] => {
        // take the permissions the earlier release held in the scope
        const held = new Set((asked[scope] ?? []).map((each) => PermissionReference.key(each)));

        // grant each new one
        return (asking[scope] ?? [])
            .filter((each) => !held.has(PermissionReference.key(each)))
            .map((each) => ({
                action: "grant",
                target: Address.join(ACCESS_TARGET, scope, each.packageId, each.type, each.name),
                risk: "privileged",
                detail: `request ${each.name} on ${each.type}${scope === "space" ? "" : ` in its ${scope}`}`,
            }));
    });

    return { steps };
}

/** Plan raising the oldest caller release a package serves, which earlier callers lose. */
function compareSince(before: string | undefined, after: string | undefined): Plan {
    // refuse earlier callers no longer served
    const isRaised =
        before === undefined || (after !== undefined && Version.compare(after, before) > 0);
    if (after === undefined || !isRaised) {
        return { steps: [] };
    }
    const detail = `serve callers from ${after}: earlier callers stay on their release`;

    return {
        steps: [{ action: "update", target: SINCE_TARGET, risk: "backward-incompatible", detail }],
    };
}

/** Plan the mount changes between releases for the callers of earlier ones. */
function compareMounts(
    before: PackageDescription["mounts"],
    after: PackageDescription["mounts"],
): Plan {
    // plan each mount either release declares
    const names = new Set([...Object.keys(before), ...Object.keys(after)]);
    const steps = [...names].toSorted().flatMap((name): Step[] => {
        // read the mount in both releases
        const previous = before[name];
        const next = after[name];
        const target = Address.join(MOUNT_TARGET, name);

        // add a new mount
        if (previous === undefined) {
            return [{ action: "create", target, risk: "safe", detail: `add mount ${name}` }];
        }
        // remove a mount
        else if (next === undefined) {
            return [
                {
                    action: "delete",
                    target,
                    risk: "backward-incompatible",
                    detail: `remove mount ${name}`,
                },
            ];
        }
        // move a mount to another path
        else if (previous.path !== next.path) {
            return [
                {
                    action: "update",
                    target,
                    risk: "backward-incompatible",
                    detail: `move mount ${name} from ${previous.path} to ${next.path}`,
                },
            ];
        }
        // skip an unchanged mount
        else {
            return [];
        }
    });

    return { steps };
}
