import { APIError } from "better-auth/api";
import { Locale, LocaleTag, SOURCE_LOCALE } from "@destack/locale";
import { schema, TimeZone } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";
import { DisplayName } from "../object/display-name.ts";
import { ResidencyCode } from "../object/residency.ts";

/** The Better Auth route a mailed sign-in link opens, carrying the sign-up in its query. */
const LINK_PATH = "/magic-link/verify";

/** The schema of the time zone and locale a person stores. */
const SignUpPerson = schema.object({
    /** The IANA time zone the person lives in, one this runtime knows. */
    timeZone: TimeZone.refine((zone) => TimeZone.isKnown(zone), "unknown time zone"),
    /** The language and region the person reads. */
    locale: LocaleTag,
});

/** The schema of a sign-up. */
const SignUpValues = SignUpPerson.extend({
    /** The person's display name. */
    name: DisplayName,
    /** The handle the person's personal account claims. */
    handle: AccountHandle,
    /** The jurisdiction the person's personal account stores data in. */
    residencyId: ResidencyCode,
});

/** What a person signs up with. */
export type SignUp = schema.Infer<typeof SignUpValues>;

/** What a person signs up with, which creates the user and their personal account together. */
export const SignUp = Object.assign(SignUpValues, {
    /** The schema of the time zone and locale the person stores. */
    person: SignUpPerson,

    /** Read the first well-formed tag a request's Accept-Language asks for. */
    locale(headers: Headers): LocaleTag {
        return firstLocale(headers.get("accept-language") ?? "") ?? SOURCE_LOCALE;
    },

    /** Read a sign-up from its fields. */
    require(values: Readonly<Record<string, unknown>>): SignUp {
        // refuse a missing or unknown time zone or locale
        const person = { timeZone: values["timeZone"], locale: values["locale"] };
        if (!SignUpPerson.safeParse(person).success) {
            throw new APIError("BAD_REQUEST", {
                code: "INVALID_SIGN_UP",
                message: "a sign-up needs a known time zone and a locale",
            });
        }

        // refuse a missing or overlong name
        const name = DisplayName.safeParse(values["name"]);
        if (!name.success) {
            throw new APIError("BAD_REQUEST", {
                code: "INVALID_SIGN_UP",
                message: "a sign-up needs a name",
            });
        }

        // refuse a missing or malformed handle or residency
        const parsed = SignUpValues.safeParse({
            ...person,
            name: name.data,
            handle: values["handle"],
            residencyId: values["residencyId"],
        });
        if (!parsed.success) {
            throw new APIError("BAD_REQUEST", {
                code: "INVALID_SIGN_UP",
                message: "a sign-up needs a handle and a residency",
            });
        }

        return parsed.data;
    },

    /** Add a sign-up to the query of a mailed sign-in link. */
    link(url: string, values: Readonly<Record<string, unknown>>): string {
        // add the sign-up to the link's query
        const signUp = SignUp.require(values);
        const link = new URL(url);
        for (const [name, value] of Object.entries(signUp)) {
            link.searchParams.set(name, value);
        }

        return link.href;
    },

    /** Read a provider's well-formed name, zoneinfo and locale claims. */
    claimed(
        profile: object,
        state: Readonly<Record<string, unknown>>,
    ): Record<keyof SignUp, unknown> {
        // read the claims a sign-up takes
        const name = "name" in profile ? DisplayName.safeParse(profile.name) : undefined;
        const zone =
            "zoneinfo" in profile
                ? SignUpPerson.shape.timeZone.safeParse(profile.zoneinfo)
                : undefined;
        const tag =
            "locale" in profile && typeof profile.locale === "string"
                ? firstLocale(profile.locale)
                : undefined;

        return {
            name: name?.success === true ? name.data : state["name"],
            timeZone: zone?.success === true ? zone.data : state["timeZone"],
            locale: tag ?? state["locale"],
            handle: state["handle"],
            residencyId: state["residencyId"],
        };
    },

    /** Read a new user's sign-up. */
    of(
        person: Readonly<Record<string, unknown>>,
        context: { readonly path?: string; readonly request?: Request | undefined } | null,
    ): SignUp {
        // read the query of a mailed sign-in link
        if (context?.path === LINK_PATH && context.request !== undefined) {
            const query = new URL(context.request.url).searchParams;

            return SignUp.require(
                Object.fromEntries(
                    Object.keys(SignUpValues.shape).map((name) => [name, query.get(name)]),
                ),
            );
        }
        // read the fields the sign-up passed
        else {
            return SignUp.require(person);
        }
    },
});

/** Read the first well-formed tag of an Accept-Language header or a single tag. */
function firstLocale(header: string): LocaleTag | undefined {
    return Locale.accepted(header).find((tag) => LocaleTag.safeParse(tag).success);
}
