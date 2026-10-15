import { aligned, TimeZone } from "@destack/schema";
import { Contact, INVITATION_PAGE, Invitation, principal } from "@destack/access";
import { Scope } from "@destack/sync";
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
            home: (call) => Account.homeOf(call, call.target.id),
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
    const from = target.login ?? aligned(target.email.split("@"), 0);
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
        const free = candidates.find((_, index) => !claimed.has(aligned(keys, index)));
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

/** List the pending invitations addressed to a user, or to the user's email once verified. */
async function invited(
    context: MethodContextOf<typeof user, "invited">,
): Promise<ResultOf<typeof user, "invited">> {
    // address the user and the email's contact
    const { target } = context;
    const person = principal.user.reference(Scope.universe.id, target.id);
    const contacts =
        target.email === null || target.emailVerified !== true
            ? []
            : [Contact.subject(Contact.email(target.email))];

    // read a page of the pending invitations to either
    const items = await Invitation.addressed(
        context,
        [person, ...contacts],
        { limit: INVITATION_PAGE },
        context.now,
    );

    return { items };
}
