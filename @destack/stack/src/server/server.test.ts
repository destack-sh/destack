import { principal } from "@destack/access";
import { ObjectType, type PackageHandle } from "@destack/object";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import manifest from "../../package.json" with { type: "json" };
import stackPackage from "../package.ts";
import { SPACE_KERNEL } from "../declare/index.ts";

/** A workspace package's manifest module, whose exports list `./package` for installable ones. */
const Module = schema.looseObject({
    default: schema.looseObject({ exports: schema.record(schema.string(), schema.unknown()) }),
});

/** A package's `./package` module, its handle as the default export. */
const Handle = schema.looseObject({ default: schema.unknown() });

/** Report whether a type's rows live in a space: in any scope, or in a space among its scopes. */
function isInSpace(type: ObjectType): boolean {
    const { packageId, name } = principal.space.definition;
    const scopes = [type.scope].flat();

    return (
        scopes.length === 0 ||
        scopes.some((scope) => {
            const policy = scope instanceof ObjectType ? scope.policy : scope;

            return (
                typeof policy !== "string" &&
                policy.definition.packageId === packageId &&
                policy.definition.name === name
            );
        })
    );
}

test("serve as a space's kernel the installer and each package it depends on serving its own types stored in a space", async () => {
    // read the object packages stack depends on in the workspace, by their manifests' package and server exports
    const installable: PackageHandle[] = [];
    const workspace = Object.keys(manifest.dependencies).filter((name) =>
        name.startsWith("@destack/"),
    );
    for (const name of workspace) {
        const directory = name.slice(name.indexOf("/") + 1);
        const read: unknown = await import(`../../../${directory}/package.json`, {
            with: { type: "json" },
        });
        const { exports } = Module.parse(read).default;
        if (Object.hasOwn(exports, "./package") && Object.hasOwn(exports, "./server")) {
            // take the handle of an object package serving itself, leaving a contract another package serves
            const imported: unknown = await import(`${name}/package`);
            const handle = Handle.parse(imported).default;
            if (isObjectPackage(handle)) {
                installable.push(handle);
            }
        }
    }

    // derive the kernel: the installer and the dependencies serving their own types stored in a space
    const derived = [
        stackPackage,
        ...installable.filter((handle) =>
            Object.values<unknown>(handle.objects).some(
                (object) => ObjectType.is(object) && isInSpace(object),
            ),
        ),
    ];
    expect([names([stackPackage, ...SPACE_KERNEL]), names(derived)]).toEqual([
        ["@destack/object", "@destack/setting", "@destack/stack"],
        ["@destack/object", "@destack/setting", "@destack/stack"],
    ]);
});

/** List packages' names in order. */
function names(handles: readonly Pick<PackageHandle, "name">[]): string[] {
    return handles.map((handle) => handle.name).toSorted();
}

/** Report whether a handle is an object package's, which lists its object types. */
function isObjectPackage(handle: unknown): handle is PackageHandle {
    return typeof handle === "object" && handle !== null && "objects" in handle;
}
