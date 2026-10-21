import { nth, present, TimeZone, zip } from "@destack/schema";
import {
    type MethodContextOf,
    type NextOf,
    type ResultOf,
    Claim,
    claimTable,
} from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { Account, account } from "../object/account.ts";
import { HANDLE_LENGTH, type HandleAvailability } from "../address/handle.ts";
import { user } from "../object/user.ts";
import { client } from "../object/client.ts";
import { Recipient } from "../object/recipient.ts";
import { invited } from "./invitation.ts";

/** The candidate handles one suggestion reads at once. */
const SUGGESTION_BATCH = 100;

/** The handle suggested when nothing else yields one. */
const DEFAULT_HANDLE_STEM = "user";

/** Implement users. */
export function implementUser(): typeof user {
    return user.implement({
        methods: {
            update,
            suggestHandle,
            checkHandle,
            invited,
            recipient: async (call) => {
                // read the person's guarded profile and unrevoked clients, which only account reveals
                const { id } = call.target;
                const person = await call.reveal(user).findFirst({ where: { id } });
                const clients = await call.reveal(client).findMany({ where: { scope: id } });

                return Recipient.of(present(person, `the profile of ${id}`), clients);
            },
        },
    });
}

/** Change a user. */
async function update(
    context: MethodContextOf<typeof user, "update">,
    next: NextOf<typeof user, "update">,
): Promise<ResultOf<typeof user, "update">> {
    // refuse an unknown time zone
    const timeZone = context.input.timeZone;
    if (typeof timeZone === "string" && !TimeZone.isKnown(timeZone)) {
        throw new CallError("BAD_REQUEST", { message: `unknown time zone ${timeZone}` });
    }

    return next();
}

/** Suggest a free handle to a user, past the ones accounts claim. */
async function suggestHandle(
    context: MethodContextOf<typeof user, "suggestHandle">,
): Promise<{ handle: string }> {
    // make the stem a valid handle
    const { target } = context;
    if (target.email === null) {
        throw new TypeError(`user ${target.id} has its email concealed`);
    }
    const from = target.login ?? nth(target.email.split("@"), 0);
    const stem =
        from
            .toLowerCase()
            .replace(/[^a-z0-9]+/gu, "-")
            .slice(0, HANDLE_LENGTH)
            .replace(/^-+|-+$/gu, "") || DEFAULT_HANDLE_STEM;

    // read which of the stem and its numbers are claimed, by a trashed account too
    for (let first = 1; ; first += SUGGESTION_BATCH) {
        const candidates = Array.from({ length: SUGGESTION_BATCH }, (_, index) =>
            numbered(stem, first + index),
        );
        const keys = candidates.map((candidate) => Claim.key(account, "handle", [candidate]).key);
        const claimed = await Claim.claimed(context.read(claimTable), {
            index: Claim.index(account, "handle"),
            keys,
        });
        const free = zip(candidates, keys).find(([, key]) => !claimed.has(key))?.[0];
        if (free !== undefined) {
            return { handle: free };
        }
    }
}

/** Number a handle stem within the handle length. */
function numbered(stem: string, number: number): string {
    const suffix = number === 1 ? "" : `-${number}`;
    const trimmed = stem.slice(0, HANDLE_LENGTH - suffix.length).replace(/-+$/u, "");

    return `${trimmed}${suffix}`;
}

/** Report to a user whether a handle is free, claimed by an account, or not a handle at all. */
async function checkHandle(
    context: MethodContextOf<typeof user, "checkHandle">,
): Promise<{ handle: string; availability: HandleAvailability }> {
    const { handle } = context.input;

    return { handle, availability: await Account.availability(context.read(claimTable), handle) };
}
