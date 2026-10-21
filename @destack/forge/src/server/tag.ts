import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import type { Identifier } from "@destack/schema";
import { CallError } from "@destack/object";
import { release, tag, TagName } from "../object/index.ts";

/** Implement distribution tags. */
export function implementTag(): typeof tag {
    return tag.implement({
        methods: {
            create: (call, next) => TagMethod.create(call, next),
            update: (call, next) => TagMethod.update(call, next),
        },
    });
}

/** The tag methods' handlers: creating and moving a tag to a published version. */
const TagMethod = {
    /** Create a tag no range reads as one, pointing at a published release. */
    async create(
        context: MethodContextOf<typeof tag, "create">,
        next: NextOf<typeof tag, "create">,
    ): Promise<ResultOf<typeof tag, "create">> {
        // require a tag name and a published target
        TagName.require(context.input.name);
        await TagMethod.requirePublished(context, context.input.parentId, context.input.version);

        return next();
    },

    /** Move a tag only to a published release. */
    async update(
        context: MethodContextOf<typeof tag, "update">,
        next: NextOf<typeof tag, "update">,
    ): Promise<ResultOf<typeof tag, "update">> {
        // require a published target when the tag moves
        const version = context.input.version;
        if (version !== undefined) {
            await TagMethod.requirePublished(context, context.target.parentId, version);
        }

        return next();
    },

    /** Require a version a tag may point at: a published release of the package. */
    async requirePublished(
        context: MethodContext,
        packageId: Identifier<"package">,
        version: string,
    ): Promise<void> {
        const [found] = await context
            .read(release)
            .findMany({ where: { parentId: packageId, version, yankedAt: { isNull: true } } });
        if (found === undefined) {
            throw new CallError("NOT_FOUND", {
                message: "distribution tag target is unpublished",
            });
        }
    },
};
