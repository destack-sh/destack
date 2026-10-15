import type { CallOf, ResultOf } from "@destack/object";
import { anyone } from "@destack/access";
import { Claim, claimTable } from "@destack/object";
import { Scope } from "@destack/sync";
import { Account, account } from "../object/account.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { machine } from "../object/machine.ts";
import { Residency, residency } from "../object/residency.ts";
import { Platform } from "./platform.ts";

/** Implement residencies. */
export function implementResidency(): typeof residency {
    return residency.implement({
        methods: {
            // show a new residency to everyone choosing one while signing up
            create: {
                creation: () => ({
                    relationships: [{ relation: "public", subject: anyone.reference("*", "*") }],
                }),
            },
            find,
            machines,
            publisher,
        },
    });
}

/** Find the residency an account keeps its data in, none for an unknown account. */
function find(call: CallOf<typeof residency, "find">): Promise<ResultOf<typeof residency, "find">> {
    return Account.findResidency(call, call.input.account);
}

/** List a residency's enabled shared cloud machines with their URLs, in identity order. */
async function machines(
    call: CallOf<typeof residency, "machines">,
): Promise<ResultOf<typeof residency, "machines">> {
    // read the residency's enabled shared machines and the endpoints they published
    const serving = await call.read(machine).findMany({
        where: {
            scope: Scope.universe.id,
            residencyId: call.target.id,
            status: "enabled",
            revokedAt: { isNull: true },
        },
        orderBy: { id: "asc" },
    });
    const published = await call
        .read(endpoint)
        .findMany({ where: { id: { in: serving.map((row) => row.id) } } });

    // keep the machines that answer at a URL now
    return serving.flatMap((row) => {
        const found = published.find((entry) => entry.id === row.id);

        return found === undefined || !Endpoint.isAnswering(found, call.now)
            ? []
            : [{ machine: row.id, url: found.url }];
    });
}

/** Find the account selling a handle's packages to the buyers of a residency's home. */
async function publisher(
    call: CallOf<typeof residency, "publisher">,
): Promise<ResultOf<typeof residency, "publisher">> {
    // find the handle's holder, none for a free handle
    const holder = await Claim.owner(
        call.read(claimTable),
        Claim.key(account, "handle", [call.input.handle]),
    );
    if (holder === undefined) {
        return null;
    }

    // sell a name or account of the platform's organisation through the residency home's account
    const platform = await Platform.find(call);
    const isPlatform = holder.objectId === platform || holder.owner === platform;
    if (isPlatform) {
        return (await Residency.isHome(call, call.input.home))
            ? Account.ofHome(call.input.home)
            : null;
    }

    return account.identifies(holder.objectId) ? account.identifier(holder.objectId) : null;
}
