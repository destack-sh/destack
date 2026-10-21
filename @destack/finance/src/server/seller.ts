import { account } from "@destack/account/object";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { JsonValue } from "@destack/schema";
import { CallError } from "@destack/object";
import { seller } from "../object/index.ts";
import type { Billing } from "./billing.ts";

/** Implement sellers: the platform's own sells at once, others through connected accounts. */
export function implementSeller(billing: Billing): typeof seller {
    return seller.implement({
        declaration: {
            // declare the platform's own seller active on the provider account itself
            resolve: async (_name, declared, stack) => {
                const accountId = account.identifier(await stack.enclosing());
                const status = billing.isPlatform(accountId) ? { status: "active" } : {};

                return { ...JsonValue.object(declared), ...status };
            },
            // retain a seller its stack stops declaring
            retire: "retained",
        },
        methods: {
            create: (call, next) => create(call, next, billing),
            onboard: {
                prepare: (context) => prepareOnboard(context, billing),
                handle: onboard,
            },
        },
    });
}

/** Create a seller, activating the platform's own at once. */
async function create(
    context: MethodContextOf<typeof seller, "create">,
    next: NextOf<typeof seller, "create">,
    billing: Billing,
): Promise<ResultOf<typeof seller, "create">> {
    // activate the platform's own seller on the provider account itself
    const created = await next();
    if (!billing.isPlatform(created.accountId)) {
        return created;
    }
    const [activated] = await context.mutate(context.self, [
        seller.call("activate", { id: created.id }),
    ]);

    return activated;
}

/** Open a seller's connected account once in its country, then its onboarding page. */
async function prepareOnboard(
    context: MethodContextOf<typeof seller, "onboard", unknown>,
    billing: Billing,
): Promise<{ readonly url: string; readonly providerId: string; readonly country: string }> {
    // refuse the platform's own seller without a connected account
    const row = context.target;
    if (billing.isPlatform(row.accountId)) {
        throw new CallError("PRECONDITION_FAILED", {
            message: "the platform sells on the provider account itself",
        });
    }

    // refuse another country than the one the connected account opened in
    const { country, returnUrl, refreshUrl } = context.input;
    if (row.country !== null && row.country !== country) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `seller ${row.id} does business from ${row.country}`,
        });
    }

    // open the connected account once, then its onboarding page
    const provider = billing.requireProvider();
    const providerId = row.providerId ?? (await provider.openSeller(country, `seller/${row.id}`));
    const url = await provider.onboard(providerId, { returnUrl, refreshUrl });

    return { url, providerId, country };
}

/** Record a connected account opened for a seller's onboarding, answering its page. */
async function onboard(
    context: MethodContextOf<typeof seller, "onboard">,
): Promise<ResultOf<typeof seller, "onboard">> {
    // record a connected account opened now, with its country
    const { url, providerId, country } = context.prepared;
    const row = context.target;
    if (row.providerId === null) {
        await context.mutate(context.self, [
            seller.call("record", { id: row.id, providerId, country }),
        ]);
    }

    return { url };
}
