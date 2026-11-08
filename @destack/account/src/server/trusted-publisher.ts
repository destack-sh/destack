import type { MethodContext } from "@destack/object";
import { Account, account } from "../object/account.ts";
import { type TrustedPublisher, trustedPublisher } from "../object/trusted-publisher.ts";
import { type RelatedFields, StoreRelationship } from "./relationship.ts";

/** Implement trusted publishers, each a publisher of its account while it exists. */
export function implementTrustedPublisher(): typeof trustedPublisher {
    return trustedPublisher.implement({
        methods: {
            create: relate,
            declare: relate,
            delete: async (call, next) => {
                // revoke the removed trusted publisher's publishing under its account's name
                await StoreRelationship.delete(call, await relationship(call, call.target));

                return next();
            },
        },
    });
}

/** Let a created or declared trusted publisher publish under its account's name. */
async function relate(
    call: MethodContext,
    next: () => Promise<TrustedPublisher>,
): Promise<TrustedPublisher> {
    const created = await next();
    await StoreRelationship.touch(call, await relationship(call, created));

    return created;
}

/** Read the relationship letting a trusted publisher publish under its account's name. */
async function relationship(context: MethodContext, row: TrustedPublisher): Promise<RelatedFields> {
    const owner = await Account.of(context, row.scope);

    return {
        object: account.reference(owner.scope, owner.id),
        relation: "publisher",
        subject: trustedPublisher.reference(row.scope, row.id),
    };
}
