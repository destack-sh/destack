import { permission, type AccessExpression } from "@destack/access";
import type { Table } from "@destack/db";
import type { Attachment } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** Objects taking attachments of other types. */
export const attachments: Trait<readonly Attachment[]> = {
    key: "attachments",
    isDurable: true,
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

        // keep attachment aggregates and dependents
        return {
            aggregates: durable.flatMap((attachment) =>
                Object.entries(attachment.object.aggregates)
                    .filter(([, aggregate]) => aggregate.via === undefined)
                    .map(([name, aggregate]) => ({
                        from: () => attachment.object.table as Table,
                        column: name,
                        key: "parentId",
                        function: aggregate.function,
                        ...(aggregate.value === undefined ? {} : { value: aggregate.value }),
                        where: { ...aggregate.where, ...hosted },
                    })),
            ),
            dependents: durable.map((attachment) => ({
                from: () => attachment.object.table as Table,
                key: "parentId",
                where: hosted,
                onDelete: attachment.object.parent!.delete,
            })),
        };
    },
    policy: (options, object, permissions) => {
        // derive each attachment's receive permission
        const attaching: Record<string, AccessExpression> = {};
        for (const attachment of options) {
            const receive = attachment.object.parent!.receive;
            const plural = attachment.object.plural;
            // refuse a permission the host lacks
            if (!permissions.includes(attachment.by)) {
                throw new TypeError(
                    `object ${object.name} attaches ${plural} by missing permission ${attachment.by}`,
                );
            }
            // require the host's own receive permission to be the attaching one
            else if (permissions.includes(receive)) {
                if (attachment.by !== receive) {
                    throw new TypeError(
                        `object ${object.name} declares permission ${receive}, so it attaches ${plural} by ${receive}`,
                    );
                }
            }
            // refuse a second attachment deriving the same permission
            else if (Object.hasOwn(attaching, receive)) {
                throw new TypeError(
                    `object ${object.name} already derives permission ${receive}, which attaching ${plural} derives`,
                );
            }
            // derive the receive permission
            else {
                attaching[receive] = permission(attachment.by);
            }
        }

        // join each attachment type's open parent relation
        return {
            permissions: attaching,
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
