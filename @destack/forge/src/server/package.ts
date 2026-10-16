import type { MethodContextOf, ResultOf } from "@destack/object";
import type { Identifier } from "@destack/schema";
import { type ObjectReference, Scope } from "@destack/sync";
import { packageObject, tag } from "../object/index.ts";
import { claimFollow, Forge } from "./forge.ts";

/** The package methods' handlers: resolving a global name and pointing a tag. */
const PackageMethod = {
    /** Find the package of a global name through the claims the forge follows on demand. */
    find(
        context: MethodContextOf<typeof packageObject, "resolve", ObjectReference | undefined>,
        home: Identifier<"space">,
    ): Promise<ObjectReference> {
        return Forge.named(
            context.input.name,
            home,
            (claim) =>
                context.acquire(claimFollow, { key: { scope: Scope.universe.id, ...claim } }),
            context,
        );
    },

    /** Read the found package as the caller, hiding one the caller may not read. */
    async resolve(
        context: MethodContextOf<typeof packageObject, "resolve">,
    ): Promise<ResultOf<typeof packageObject, "resolve">> {
        return Forge.conceal("package not found", () =>
            context.query(
                context.caller,
                packageObject.call("get", packageObject.target(context.prepared)),
            ),
        );
    },

    /** Point a package's distribution tag at a version, creating the tag the first time. */
    async point(
        context: MethodContextOf<typeof packageObject, "point">,
    ): Promise<ResultOf<typeof packageObject, "point">> {
        // find the package's tag of the name
        const { name, version } = context.input;
        const parentId = context.target.id;
        const [existing] = await context.read(tag).findMany({ where: { parentId, name } });

        // create the tag
        if (existing === undefined) {
            await context.mutate(context.caller, [tag.call("create", { parentId, name, version })]);
        }
        // move the tag
        else {
            await context.mutate(context.caller, [
                tag.call("update", { id: existing.id, version }),
            ]);
        }

        return context.target;
    },
};

/** Implement packages of the forge's space. */
export function implementPackage(home: Identifier<"space">): typeof packageObject {
    return packageObject.implement({
        methods: {
            resolve: {
                prepare: (context) => PackageMethod.find(context, home),
                handle: (context) => PackageMethod.resolve(context),
            },
            point: (context) => PackageMethod.point(context),
        },
    });
}
