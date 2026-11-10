import type { BetterAuthPlugin } from "better-auth";
import { APIError, createAuthEndpoint } from "better-auth/api";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { claimTable } from "@destack/object";
import { Locale, LocaleTag, SOURCE_LOCALE } from "@destack/locale";
import { schema, TimeZone } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";
import { DisplayName } from "../object/display-name.ts";
import { Residency, ResidencyCode } from "../object/residency.ts";
import { Account } from "../object/account.ts";
import { HANDLE_CHECK_PATH, RESIDENCY_CHOICE_PATH } from "../authentication/path.ts";
import { HandleCheck, ResidencyChoice } from "../authentication/request.ts";

/** The header Cloudflare's edge reads the visitor's country into, ISO 3166-1 alpha-2. */
const COUNTRY_HEADER = "cf-ipcountry";

/** The query of a handle's check: the handle as the person typed it. */
const HandleQuery = schema.object({
    /** The handle as typed. */
    handle: schema.string().max(64),
});

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

    /** Answer what a sign-up asks before the person exists: handles and residencies. */
    plugin(database: DatabaseConnection): BetterAuthPlugin {
        return {
            id: "destack-sign-up",
            endpoints: {
                /** Report whether a handle is free, taken, or invalid. */
                checkHandle: createAuthEndpoint(
                    HANDLE_CHECK_PATH,
                    { method: "GET", query: HandleQuery },
                    async (context) => {
                        const { handle } = context.query;
                        const claims = Snapshot.live(database).read(claimTable);

                        return context.json(
                            HandleCheck.parse({
                                handle,
                                availability: await Account.availability(claims, handle),
                            }),
                        );
                    },
                ),

                /** List the residencies, and the one offered the visitor's country, if any. */
                listResidencies: createAuthEndpoint(
                    RESIDENCY_CHOICE_PATH,
                    { method: "GET" },
                    async (context) => {
                        // offer the residency listing the edge's country of the visitor
                        const residencies = await Residency.list(database);
                        const country = context.request?.headers.get(COUNTRY_HEADER)?.toUpperCase();
                        const suggested = residencies.find(
                            (each) => country !== undefined && each.countries.includes(country),
                        );

                        return context.json(
                            ResidencyChoice.parse({
                                residencies: residencies.map(({ code, name }) => ({ code, name })),
                                suggested: suggested?.code ?? null,
                            }),
                        );
                    },
                ),
            },
        };
    },

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
});

/** Read the first well-formed tag of an Accept-Language header or a single tag. */
function firstLocale(header: string): LocaleTag | undefined {
    return Locale.accepted(header).find((tag) => LocaleTag.safeParse(tag).success);
}
