import { present } from "@destack/schema";
import { permission, subjectSet, union, type AccessExpression } from "@destack/access";

import type { Attachment } from "../object/object.ts";
import type { Trait } from "./trait.ts";
import { installedPackage } from "../installation/package.ts";

/** Objects taking attachments of other types. */
export const attachments: Trait<readonly Attachment[]> = {
    key: "attachments",
    storages: ["durable"],
    options: (definition) =>
        definition.attachments === undefined || definition.attachments.length === 0
            ? undefined
            : definition.attachments,
    columns: () => ({}),
    constraints: () => [],
    table: (options, object) => {
        // select durable attachments hosted by this type
        const hosted = { parentPackageId: object.packageId, parentType: object.name };
        const durable = options.filter((attachment) => attachment.object.storage === "durable");

        // add attachment aggregates and dependents
        return {
            aggregates: durable.flatMap((attachment) =>
                Object.entries(attachment.object.aggregates)
                    .filter(([, aggregate]) => aggregate.via === undefined)
                    .map(([name, aggregate]) => ({
                        from: () => attachment.object.table,
                        column: name,
                        key: "parentId",
                        ...(aggregate.function === "count"
                            ? { function: aggregate.function }
                            : { function: aggregate.function, value: aggregate.value }),
                        where: { ...aggregate.where, ...hosted },
                    })),
            ),
            dependents: durable.map((attachment) => {
                // delete or restrict attachments with their host, which has no trash for them
                const deletion = present(
                    attachment.object.parent,
                    "an attachment's nesting",
                ).delete;
                if (deletion === "trash") {
                    throw new TypeError(`attachments of ${object.name} cannot move to the trash`);
                }

                return {
                    from: () => attachment.object.table,
                    key: "parentId",
                    where: hosted,
                    onDelete: deletion,
                };
            }),
        };
    },
    policy: (options, object, permissions) => {
        // derive each attachment's receive permission
        const attaching: Record<string, AccessExpression> = {};
        for (const attachment of options) {
            const receive = present(attachment.object.parent, "an attachment's nesting").receive;
            const plural = attachment.object.plural;
            // refuse a permission the host lacks
            if (!permissions.includes(attachment.by)) {
                throw new TypeError(
                    `object ${object.name} attaches ${plural} by permission ${attachment.by}, which it does not declare`,
                );
            }
            // require the host's receive permission to be the attaching one
            else if (permissions.includes(receive)) {
                if (attachment.by !== receive) {
                    throw new TypeError(
                        `object ${object.name} declares permission ${receive}, so it must attach ${plural} by ${receive}`,
                    );
                }
            }
            // refuse a second attachment deriving the same permission
            else if (Object.hasOwn(attaching, receive)) {
                throw new TypeError(
                    `object ${object.name} already derives permission ${receive}, which attaching ${plural} also derives`,
                );
            }
            // derive the receive permission: the attaching one, or the attachment package's installations
            else {
                const { packageId } = attachment.object.policy.definition;
                attaching[receive] = union(
                    permission(attachment.by),
                    subjectSet(installedPackage, packageId, "installation"),
                );
            }
        }

        // join each attachment type's open parent relation, granting the package sets it derives
        const isGranting = Object.values(attaching).length > 0;

        return {
            permissions: attaching,
            sets: isGranting ? [installedPackage] : [],
            contributes: options.map((attachment) => ({
                policy: attachment.object.policy,
                relation: "parent",
            })),
        };
    },
    methods: () => ({}),
    validate: (options, object) => {
        // require the attachments' parent aggregates
        for (const attachment of options) {
            for (const [name, aggregate] of Object.entries(attachment.object.aggregates)) {
                if (aggregate.via === undefined) {
                    object.requireAggregate(attachment.object, name, aggregate);
                }
            }
        }
    },
};
