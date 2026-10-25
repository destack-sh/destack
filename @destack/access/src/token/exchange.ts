import { Caller } from "../context/index.ts";
import { principal } from "../declare/index.ts";
import { JsonValue } from "@destack/schema";
import { AccessError } from "../error/index.ts";

/** The schema of the caller an instance's call acts as: its subject and grants. */
const ActingCaller = Caller.schema.pick({ subject: true, grants: true });

/** The exchange of an installation's token for its caller's (RFC 8693). */
export const Exchange = {
    /** The header with the caller an instance's call asks its host's egress to act as. */
    header: "Destack-Exchange",

    /** Read the caller a served call's calls act as: a delegated subject, or a person calling. */
    of(caller: Caller | undefined): Pick<Caller, "subject" | "grants"> | undefined {
        // act for the subject under the grants of a delegated call
        const subject = caller?.subject;
        if (caller !== undefined && subject !== undefined && caller.grants.length > 0) {
            return { subject, grants: caller.grants };
        }
        // act for a person calling directly
        else if (subject !== undefined && principal.user.is(subject)) {
            return { subject, grants: [] };
        }

        return undefined;
    },

    /** Write the header asking the egress to send a call as a caller. */
    ask(caller: Pick<Caller, "subject" | "grants"> | undefined): Readonly<Record<string, string>> {
        return caller === undefined ? {} : { [Exchange.header]: JSON.stringify(caller) };
    },

    /** Read the caller a call asks the egress to act as, absent for an installation as itself. */
    asking(request: Request): Pick<Caller, "subject" | "grants"> | undefined {
        // ask nothing without the header
        const header = request.headers.get(Exchange.header);
        if (header === null) {
            return undefined;
        }

        // read the subject and its grants, refusing a malformed header
        const asked = ActingCaller.safeParse(JsonValue.parse(header));
        if (!asked.success) {
            throw AccessError.of("EXCHANGE_INVALID", `invalid ${Exchange.header}`);
        }

        return asked.data;
    },
};
