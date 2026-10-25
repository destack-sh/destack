import { and, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { present, schema } from "@destack/schema";
import { USER_INFO_PATH } from "../authentication/path.ts";
import { OAuthError } from "../authentication/token.ts";
import { SignUp } from "../better-auth/sign-up.ts";
import { account } from "../object/account.ts";
import { user, User } from "../object/user.ts";
import { AccessToken, type AccessTokenOptions, spaceSeparated } from "./token.ts";

/** The claims of a userinfo request's token: the person and the granted scopes. */
const UserInfoClaims = schema.looseObject({
    /** The person. */
    sub: schema.identifier("user"),
    /** The granted scopes, space separated. */
    scope: schema.string(),
});

/** What the userinfo endpoint verifies tokens and reads people with. */
export interface UserInfoOptions extends AccessTokenOptions {
    /** The account database with the people. */
    readonly database: DatabaseConnection;
}

/** The userinfo endpoint: a person's claims to a client with their access token (OIDC Core 5.3). */
export class UserInfoEndpoint {
    /** What the endpoint verifies tokens and reads people with. */
    readonly #options: UserInfoOptions;

    /** Answer userinfo requests. */
    constructor(options: UserInfoOptions) {
        this.#options = options;
    }

    /** Answer the claims of the person a request's access token is for, as its scopes allow. */
    async answer(request: Request): Promise<Response> {
        // verify the bearer token for the userinfo endpoint
        const { database, issuer } = this.#options;
        const token = /^bearer (?<token>\S+)$/iu.exec(request.headers.get("authorization") ?? "")
            ?.groups?.["token"];
        if (token === undefined) {
            throw OAuthError.of("INVALID_TOKEN", "the request has no bearer token");
        }
        const verified = await AccessToken.verify(
            token,
            `${issuer}${USER_INFO_PATH}`,
            this.#options,
        );
        const claims = UserInfoClaims.safeParse(verified);
        if (!claims.success) {
            throw OAuthError.of("INVALID_TOKEN", "the token has no person");
        }
        const { sub, scope } = claims.data;
        const scopes = spaceSeparated(scope);

        // read the person, refusing one who may no longer sign in
        const person = await Snapshot.live(database)
            .read(user.table)
            .findFirst({ where: { id: sub } });
        if (person === undefined || !(await User.maySignIn(database, sub))) {
            throw OAuthError.of("INVALID_TOKEN", "the user may no longer sign in");
        }

        // answer the profile and email the scopes grant (OIDC Core 5.4)
        const profile = scopes.includes("profile") ? await profileClaims(person, database) : {};
        const email = scopes.includes("email")
            ? { email: person.email, email_verified: person.emailVerified }
            : {};

        return Response.json({ sub, ...profile, ...email });
    }
}

/** Read a person's profile claims: their name, picture, handle, locale and time zone. */
async function profileClaims(
    person: Pick<User, "id" | "name" | "image" | "timeZone" | "locale">,
    database: DatabaseConnection,
) {
    // read the time zone and locale, and the handle of the personal account
    const { timeZone, locale } = SignUp.person.parse({
        timeZone: person.timeZone,
        locale: person.locale,
    });
    const [personal] = await database
        .select({ handle: account.table.handle })
        .from(account.table)
        .where(and(eq(account.table.scope, person.id), eq(account.table.kind, "personal")));

    return {
        name: person.name,
        picture: person.image ?? undefined,
        preferred_username: present(personal, `the personal account of ${person.id}`).handle,
        locale,
        zoneinfo: timeZone,
    };
}
