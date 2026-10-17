import { Caller, principal } from "@destack/access";
import { defineSchema, JsonValue, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { CallError } from "../error/index.ts";

/** What an instance's call asks its host's egress to exchange its token for (RFC 8693). */
export const ExchangeAsk = defineSchema(
    schema.object({
        /** The principal the call acts for. */
        subject: Subject,
        /** The grants of the chain the call acts under, else the subject's one grant to it. */
        grants: schema.array(schema.identifier("grant")).min(1).exactOptional(),
    }),
);
/** What an instance's call asks its host's egress to exchange its token for. */
export type ExchangeAsk = schema.Infer<typeof ExchangeAsk>;

/** The context of a call an instance makes, for the subject of a call it serves when given. */
export interface ExchangeContext {
    /** The exchange the call asks for, absent for the installation's own call. */
    readonly exchange?: ExchangeAsk;
}

/** The exchange of an installation's token for its subject's, under the subject's grant to it. */
export const Exchange = {
    /** The header an instance's call asks its host's egress for an exchange with. */
    header: "Destack-Exchange",

    /** Read the exchange of calls for a served call's person, or its subject under its grant. */
    of(caller: Caller | undefined): ExchangeAsk | undefined {
        // act for the subject under the grants of a delegated call
        const subject = caller?.subject;
        if (subject !== undefined && caller !== undefined && caller.grants.length > 0) {
            return { subject, grants: caller.grants.map((grant) => grant.id) };
        }
        // act for a person calling directly
        else if (subject !== undefined && principal.user.is(subject)) {
            return { subject };
        }

        return undefined;
    },

    /** Write the headers asking the egress to send a call for a subject. */
    ask(exchange: ExchangeAsk | undefined): Readonly<Record<string, string>> {
        return exchange === undefined ? {} : { [Exchange.header]: JSON.stringify(exchange) };
    },

    /** Read the exchange a call asks the egress for, absent for a call asking none. */
    asking(request: Request): ExchangeAsk | undefined {
        // ask nothing without the header
        const header = request.headers.get(Exchange.header);
        if (header === null) {
            return undefined;
        }

        // read the subject and the grant, refusing a malformed ask
        const asked = ExchangeAsk.safeParse(JsonValue.parse(header));
        if (!asked.success) {
            throw new CallError("BAD_REQUEST", { message: `invalid ${Exchange.header}` });
        }

        return asked.data;
    },
};
