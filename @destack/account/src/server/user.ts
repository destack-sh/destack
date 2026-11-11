import { Subject } from "@destack/sync";
import { nth, present, TimeZone, zip } from "@destack/schema";
import {
    type MethodContextOf,
    type NextOf,
    type ResultOf,
    Claim,
    claimTable,
} from "@destack/object";
import { Account, account } from "../object/account.ts";
import { AccountHandle, type HandleAvailability } from "../address/handle.ts";
import { user } from "../object/user.ts";
import { client } from "../object/client.ts";
import { Recipient } from "../object/recipient.ts";
import { invited } from "./invitation.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The candidate handles one suggestion reads at once. */
const SUGGESTION_BATCH = 100;

/** Implement users. */
export function implementUser(): typeof user {
    return user.implement({
        methods: {
            update,
            suspend: (call) => {
                // refuse an operator suspending themselves, which could leave none to resume them
                if (Subject.same(call.caller.subject, call.reference())) {
                    throw AccountError.of(
                        "SELF_SUSPENSION_INVALID",
                        "suspension is invalid: an operator cannot suspend themselves",
                    );
                }

                return call.update({ suspendedAt: call.now });
            },
            resume: (call) => call.update({ suspendedAt: null }),
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
        throw AccountError.of(
            "TIME_ZONE_INVALID",
            `time zone '${timeZone}' is invalid: it is unknown`,
        );
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
        throw AccountError.of(
            "EMAIL_DENIED",
            `email of user '${target.id}' is forbidden: the user has it concealed`,
        );
    }
    const stem = AccountHandle.stem(target.login ?? nth(target.email.split("@"), 0));

    // read which of the stem and its numbers are claimed or look alike, by a trashed account too
    for (let first = 1; ; first += SUGGESTION_BATCH) {
        const candidates = AccountHandle.candidates(stem, first, SUGGESTION_BATCH);
        const keys = candidates.map(
            (candidate) => Claim.key(account, "lookalike", [candidate]).key,
        );
        const claimed = await Claim.claimed(context.read(claimTable), {
            index: Claim.index(account, "lookalike"),
            keys,
        });
        const free = zip(candidates, keys).find(([, key]) => !claimed.has(key))?.[0];
        if (free !== undefined) {
            return { handle: free };
        }
    }
}

/** Report to a user whether a handle is free, claimed by an account, or not a handle at all. */
async function checkHandle(
    context: MethodContextOf<typeof user, "checkHandle">,
): Promise<{ handle: string; availability: HandleAvailability }> {
    const { handle } = context.input;

    return { handle, availability: await Account.availability(context.read(claimTable), handle) };
}
