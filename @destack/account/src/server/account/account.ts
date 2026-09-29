import { and, eq } from "@destack/db";
import { Scope } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import type { Call } from "@destack/object";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import * as base from "../../object/account.ts";
import { organisation } from "../../object/organisation.ts";
import { user } from "../../object/user.ts";

/** One call of an account method. */
type AccountCall = Call<typeof base.account.table>;

/** Accounts on the server with their owner's residency at start. */
export const account = base.account.handle({ create });

/** Create an account with its owner's residency. */
async function create(call: AccountCall, next: (call?: AccountCall) => Promise<unknown>) {
    // require a user's personal account before any other in their scope
    const owner = await Scope.object(Snapshot.live(call.database), call.scope);
    const isUser = owner.type !== organisation.name;
    if (isUser && call.input.kind !== "personal") {
        const [personal] = await call.database
            .select({ id: base.account.table.id })
            .from(base.account.table)
            .where(
                and(
                    eq(base.account.table.scope, identifier("user").parse(owner.id)),
                    eq(base.account.table.kind, "personal"),
                ),
            );
        if (personal === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "choose a handle first to create the personal account",
            });
        }
    }

    // read the residency of the owning user or organisation
    const [owned] = isUser
        ? await call.database
              .select({ residency: user.table.residency })
              .from(user.table)
              .where(eq(user.table.id, identifier("user").parse(owner.id)))
        : await call.database
              .select({ residency: organisation.table.residency })
              .from(organisation.table)
              .where(eq(organisation.table.id, identifier("organisation").parse(owner.id)));

    // refuse a missing owner, and one without a chosen residency
    if (owned === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no account owner ${owner.id}` });
    }
    const { residency } = owned;
    if (residency === null) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: "choose a residency with the handle first",
        });
    }

    return next(
        call.with({
            input: {
                ...call.input,
                defaultResidency: call.input.defaultResidency ?? residency,
            },
        }),
    );
}
