import { eq, inArray } from "@destack/db";
import type { Call } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { account } from "../../object/account.ts";
import { AccountHandle, type HandleAvailability } from "../../object/handle.ts";
import * as base from "../../object/user.ts";
import type { User } from "../../object/user.ts";

/** The longest handle, a DNS label. */
const HANDLE_LENGTH = 63;

/** The candidate handles one suggestion reads at once, 100 keys of one indexed lookup under a millisecond. */
const SUGGESTION_BATCH = 100;

/** The handle suggested when nothing else yields one. */
const FALLBACK = "user";

/** One call of a user method. */
type UserCall = Call<typeof base.user.table>;

/** Users on the server. */
export const user = base.user.handle({ update, suggestHandle, checkHandle });

/** Change a user. */
async function update(call: UserCall, next: (call?: UserCall) => Promise<unknown>) {
    // keep the residency the user chose
    const residency = call.input.residency;
    const chosen = (call.target as User).residency;
    if (residency !== undefined && chosen !== null && residency !== chosen) {
        throw new ServiceError("CONFLICT", {
            message: "the residency follows the home space",
        });
    }

    // resolve the zone as every reader of it does
    const timeZone = call.input.timeZone;
    if (typeof timeZone === "string") {
        try {
            new Intl.DateTimeFormat("en", { timeZone });
        } catch (error) {
            throw new ServiceError("BAD_REQUEST", {
                message: `unknown time zone ${timeZone}`,
                cause: error,
            });
        }
    }

    return next();
}

/** Suggest a free handle to a user. */
async function suggestHandle(call: UserCall): Promise<{ handle: string }> {
    // make the stem a valid handle
    const target = call.target as User;
    const from = target.login ?? target.email.split("@")[0]!;
    const stem =
        from
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, "-")
            .slice(0, HANDLE_LENGTH)
            .replace(/^-+|-+$/g, "") || FALLBACK;

    // read which of the stem and its numbers are taken a batch at a time, and take the first free one
    for (let first = 1; ; first += SUGGESTION_BATCH) {
        const candidates = Array.from({ length: SUGGESTION_BATCH }, (_, index) =>
            numbered(stem, first + index),
        );
        const rows = await call.database
            .select({ handle: account.table.handle })
            .from(account.table)
            .where(inArray(account.table.handle, candidates));
        const taken = new Set(rows.map((row) => row.handle));
        const free = candidates.find((candidate) => !taken.has(candidate));
        if (free !== undefined) {
            return { handle: free };
        }
    }
}

/** Number a handle stem, the first one bare and the others with a suffix, within the handle length. */
function numbered(stem: string, number: number): string {
    const suffix = number === 1 ? "" : `-${number}`;
    const trimmed = stem.slice(0, HANDLE_LENGTH - suffix.length).replace(/-+$/, "");

    return `${trimmed}${suffix}`;
}

/** Report to a user whether a handle is free, taken by an account, or not a handle at all. */
async function checkHandle(
    call: UserCall,
): Promise<{ handle: string; availability: HandleAvailability }> {
    // refuse what is no handle
    const handle = call.input.handle as string;
    if (!AccountHandle.safeParse(handle).success) {
        return { handle, availability: "invalid" };
    }

    // find an account holding it
    const [taken] = await call.database
        .select({ id: account.table.id })
        .from(account.table)
        .where(eq(account.table.handle, handle))
        .limit(1);

    return { handle, availability: taken === undefined ? "available" : "taken" };
}
