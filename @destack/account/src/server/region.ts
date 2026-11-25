import { type MethodContextOf, type ResultOf } from "@destack/object";
import { anyone } from "@destack/access";
import { Account } from "../object/account.ts";
import { Region, region } from "../object/region.ts";

/** Implement regions. */
export function implementRegion(): typeof region {
    return region.implement({
        methods: {
            // show a new region to everyone choosing one while signing up
            create: {
                relate: () => ({
                    relationships: [{ relation: "public", subject: anyone.reference("*", "*") }],
                }),
            },
            relay,
        },
    });
}

/** Read the relay of an account's region. */
async function relay(
    context: MethodContextOf<typeof region, "relay">,
): Promise<ResultOf<typeof region, "relay">> {
    const { regionId } = await Account.of(context, context.input.account);
    const found = await Region.require(context, regionId);

    return { url: found.relay };
}
