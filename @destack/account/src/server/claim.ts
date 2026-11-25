import { Claim, claimTable, type MethodContextOf, type ResultOf } from "@destack/object";
import { Account, account } from "../object/account.ts";
import { claim } from "../object/claim.ts";

/** Implement claims. */
export function implementClaim(): typeof claim {
    return claim.implement({ methods: { holder } });
}

/** Read the account holding a handle or a lookalike, and the account paying for it. */
async function holder(
    context: MethodContextOf<typeof claim, "holder">,
): Promise<ResultOf<typeof claim, "holder">> {
    // find the claim's object, none for a free handle
    const key = Claim.key(account, "lookalike", [context.input.handle]);
    const found = await Claim.owner(context.read(claimTable), key);
    if (found === undefined) {
        return null;
    }

    // take the claiming account, a reserving organization's own, and its payer
    const holding = await Account.of(context, found.objectId);

    return { account: holding.id, payer: await Account.payer(context, holding.id) };
}
