import { Caller, type Credential } from "../context/index.ts";
import { principal } from "../declare/index.ts";
import { JsonValue, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";

/** The credential a person acts within: the session they signed in with, or their client. */
export const PersonCredential = schema.union([
    schema.identifier("session"),
    schema.identifier("client"),
]);
/** The credential a person acts within. */
export type PersonCredential = schema.Infer<typeof PersonCredential>;

/** Whom an exchanged token acts for: a subject under its grants, within a person's credential. */
export const ExchangeSubject = Caller.schema.pick({ subject: true, grants: true }).extend({
    /** The person's session or client the token lives within, ending with it. */
    credential: PersonCredential.exactOptional(),
});
/** Whom an exchanged token acts for. */
export type ExchangeSubject = schema.Infer<typeof ExchangeSubject>;

/** The exchange of an installation's token for its caller's (RFC 8693). */
export const Exchange = {
    /** The header with the caller an instance's call asks its host's egress to act as. */
    header: "Destack-Exchange",

    /** Read whom a served call's calls act as: a delegate's subject, a person, or anyone. */
    of(
        caller: Caller | undefined,
        presented?: Pick<Credential, "id">,
    ): ExchangeSubject | undefined {
        // live within the person's session or client the call presented
        const subject = caller?.subject;
        const credential = PersonCredential.safeParse(presented?.id);
        const within = credential.success ? { credential: credential.data } : {};

        // act for the subject under the grants of a delegated call
        if (caller !== undefined && subject !== undefined && caller.grants.length > 0) {
            return { subject, grants: [...caller.grants], ...within };
        }
        // act for a person calling directly
        else if (subject !== undefined && principal.user.is(subject)) {
            return { subject, grants: [], ...within };
        }
        // act for an anonymous visitor as anyone, never as the installation serving them
        else if (subject !== undefined && principal.anonymous.is(subject)) {
            return { subject, grants: [] };
        }

        return undefined;
    },

    /** Write the header asking the egress to send a call as a subject. */
    ask(subject: ExchangeSubject | undefined): Readonly<Record<string, string>> {
        return subject === undefined ? {} : { [Exchange.header]: JSON.stringify(subject) };
    },

    /** Read whom a call asks the egress to act as, absent for an installation as itself. */
    asking(request: Request): ExchangeSubject | undefined {
        // ask nothing without the header
        const header = request.headers.get(Exchange.header);
        if (header === null) {
            return undefined;
        }

        // read the subject and its grants, refusing a malformed header
        const asked = ExchangeSubject.safeParse(JsonValue.parse(header));
        if (!asked.success) {
            throw AccessError.of(
                "EXCHANGE_INVALID",
                `header '${Exchange.header}' is invalid: it must be a token exchange header`,
            );
        }

        return asked.data;
    },
};
