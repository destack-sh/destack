import { Address, Plan, type Comparator, type Step } from "@destack/resource";
import { Version } from "@destack/schema";
import { PackageDescription } from "../declare/description.ts";
import type { ObjectDeclaration } from "../declare/package.ts";
import type { Package, PackageIdentity } from "../declare/package.ts";

/** The address of the oldest caller release a package serves in its release plans. */
const SINCE_TARGET = "since";

/** The address of a package's mounts in its release plans. */
const MOUNT_TARGET = "mount";

/** Describe a package: its objects, mounts and views. */
export function describePackage(handle: Package & PackageIdentity): PackageDescription {
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

/** Plan a package's caller and mount changes between releases. */
export const comparePackage: Comparator = (earlier, later) => {
    // read both releases
    const before = PackageDescription.parse(earlier.description);
    const after = PackageDescription.parse(later.description);

    return Plan.join([
        () => compareSince(before.since, after.since),
        () => compareMounts(before.mounts, after.mounts),
    ]);
};

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
