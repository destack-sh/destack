import { Claim, claimTable, type MethodContextOf, type ResultOf } from "@destack/object";
import { account } from "../object/account.ts";
import { claim } from "../object/claim.ts";

/** Implement claims. */
export function implementClaim(): typeof claim {
    return claim.implement({ methods: { holder } });
}

/** Read the account holding a handle or a lookalike, none for a free or reserved handle. */
async function holder(
    context: MethodContextOf<typeof claim, "holder">,
): Promise<ResultOf<typeof claim, "holder">> {
    // find the claim's object, none for a free handle
    const key = Claim.key(account, "lookalike", [context.input.handle]);
    const found = await Claim.owner(context.read(claimTable), key);

    // answer an account, and none for a handle the platform's organization reserves without one
    return found !== undefined && account.identifies(found.objectId)
        ? account.identifier(found.objectId)
        : null;
}
