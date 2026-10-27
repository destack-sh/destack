import * as accountObject from "@destack/account/object";
import { type LocaleTag, SOURCE_LOCALE } from "@destack/locale";
import { principal } from "@destack/access";
import { Destination } from "@destack/message";
import type { MethodContext } from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";

/** The capability of pushing notifications to people's browsers, which an installation may lack. */
export const PUSH_CAPABILITY = "push";

/** The capability of delivering a person's notifications from their home. */
export const DELIVER_CAPABILITY = "deliver";

/** A destination a person receives notifications at: a desktop, a browser or an email address. */
export type RecipientDestination = Exclude<Destination, { readonly channel: "webhook" }>;

/** Where a person receives notifications beyond the inbox, and in which language. */
export class Recipient {
    /** The language the person reads. */
    readonly locale: LocaleTag;
    /** The person's time zone. */
    readonly timeZone: string;
    /** The destinations the person receives messages at: desktops, browsers and email. */
    readonly destinations: readonly RecipientDestination[];

    /** List a person's destinations from account's recipient row. */
    constructor(row: accountObject.Recipient, person: Identifier<"user">) {
        // take the language and time zone
        this.locale = row.locale;
        this.timeZone = row.timeZone;

        // list the desktops, the browsers with their person, and the email address
        this.destinations = [
            ...row.desktops.map((id) => ({ channel: "desktop" as const, client: id })),
            ...row.browsers.map(({ id }) => ({
                channel: "push" as const,
                client: id,
                user: person,
            })),
            ...(row.email === undefined ? [] : [{ channel: "email" as const, address: row.email }]),
        ];
    }

    /** Read the language a principal reads: a person's profile's, else the messages' own. */
    static async locale(
        context: Pick<MethodContext, "read">,
        subject: Subject,
    ): Promise<LocaleTag> {
        // read the messages as authored for a principal without a profile, such as an agent
        if (!principal.user.is(subject)) {
            return SOURCE_LOCALE;
        }

        // read a person's language from their public profile, which the space follows for its members
        const profile = await context
            .read(accountObject.user)
            .findFirst({ where: { id: accountObject.user.identifier(subject.id) } });

        return present(profile, `the profile of ${subject.id}`).locale;
    }

    /** Read where a person receives notifications through account's query, acting for the home. */
    static async read(context: MethodContext, person: Subject): Promise<Recipient> {
        const id = schema.identifier("user").parse(person.id);
        const call = accountObject.user.call("recipient", { id });

        return new Recipient(await context.query(await context.delegated(), call), id);
    }

    /** List the destinations a call may send to, pushing only where the installation may push. */
    reachable(context: MethodContext): RecipientDestination[] {
        return this.destinations.filter(
            (to) => to.channel !== "push" || context.enabled(PUSH_CAPABILITY),
        );
    }

    /** Report whether the person still receives messages at a destination. */
    addresses(to: Destination): boolean {
        return this.destinations.some((known) => Destination.same(known, to));
    }
}
