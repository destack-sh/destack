import { type Caller, CLOCK_TOLERANCE_MILLISECONDS, principal } from "@destack/access";
import type { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Scope, type Subject } from "@destack/sync";
import { jwtVerify, type JWTPayload, type JWTVerifyGetKey } from "jose";
import { ServiceError } from "../error/index.ts";
import { Bearer } from "./bearer.ts";

/** How long a Loan lends a person's authority: a minute, one call's latency with room for retries, after the AT Protocol's inter-service tokens. */
export const LOAN_MILLISECONDS = 60_000;

/** The claims of a Loan: a person's home lending their authority for one method in one other space. */
const LoanClaims = schema.object({
    /** The home space lending the authority. */
    iss: schema.identifier("space"),
    /** The person whose authority it lends. */
    sub: schema.identifier("user"),
    /** The space the call goes to. */
    aud: schema.identifier("space"),
    /** The one method lent, as `<package>/<object>.<method>`. */
    lxm: schema.string().min(1),
    /** The home's installation acting with the authority (RFC 8693 4.1), absent for the person's own call. */
    act: schema.object({ sub: schema.identifier("installation") }).exactOptional(),
    /** When it was signed, in seconds. */
    iat: schema.number().int(),
    /** When it lapses, in seconds. */
    exp: schema.number().int(),
    /** Its identifier. */
    jti: schema.string().min(1),
});

/** What an instance's call asks its host's egress to lend: the method it makes, and the token its space lent it the person's authority with. */
export const LoanAsk = defineSchema(
    schema.object({
        /** The one method the call makes, as `<package>/<object>.<method>`. */
        method: schema.string().min(1),
        /** The token the instance's space exchanged for the person's call to it (RFC 8693). */
        delegation: schema.string().min(1),
    }),
);
/** What an instance's call asks its host's egress to lend. */
export type LoanAsk = schema.Infer<typeof LoanAsk>;

/** What a home signs a Loan for: an instance's ask, for the space its call goes to. */
export const LoanRequest = defineSchema(
    LoanAsk.extend({
        /** The space the call goes to. */
        space: schema.identifier("space"),
    }),
);
/** What a home signs a Loan for. */
export type LoanRequest = schema.Infer<typeof LoanRequest>;

/** The context a call made for a served call's person carries, as a binding's client takes it per call: the delegation lending that person's authority. */
export interface LoanContext {
    /** The delegation of the person the served call acts for, absent for the installation's own call. */
    readonly delegation?: string;
}

/** What a home lends: the person, the space and the one method, and the installation acting for the person. */
export interface LoanGrant {
    /** The person's home space, which signs the Loan. */
    readonly home: string;
    /** The person whose authority it lends. */
    readonly person: string;
    /** The space the call goes to. */
    readonly space: string;
    /** The one method lent, as `<package>/<object>.<method>`. */
    readonly method: string;
    /** The installation of the home acting with the authority, absent for the person's own call. */
    readonly actor?: string;
}

/** What a Loan is verified against at its target. */
export interface LoanVerification {
    /** Read the published signing keys of a space, as the directory keeps them. */
    readonly keys: (space: string) => JWTVerifyGetKey;
    /** Read a person's home space, the one space that lends their authority elsewhere. */
    readonly home: (person: Subject) => Promise<string>;
    /** The space receiving the call. */
    readonly space: string;
    /** The package receiving the call. */
    readonly audience: PackageId;
}

/** A person's authority their home lends for one call of one method in another space, signed with the home's key and verified through the directory. */
export const Loan = {
    /** The header carrying a Loan beside a call's own credential. */
    header: "Destack-Loan",
    /** The header an instance's call asks its host's egress for a Loan with, as `<method> <delegation>`, which the egress takes off. */
    asked: "Destack-Loan-Ask",

    /** Format a method of an object type as a Loan lends it: `<package>/<object>.<method>`. */
    method(
        object: { readonly package: { readonly name: string }; readonly name: string },
        name: string,
    ): string {
        return `${object.package.name}/${object.name}.${name}`;
    },

    /** Read the context a call made for a served call's person carries: its caller's delegation, none for an undelegated caller. */
    of(caller: Pick<Caller, "delegation"> | undefined): LoanContext {
        const delegation = caller?.delegation;

        return delegation === undefined ? {} : { delegation };
    },

    /** Write the headers asking the egress for a Loan of the person a call's context names, none for a call without a delegation. */
    ask(method: string, context: LoanContext): Readonly<Record<string, string>> {
        const { delegation } = context;

        return delegation === undefined ? {} : { [Loan.asked]: `${method} ${delegation}` };
    },

    /** Read the Loan a call asks the egress for, absent for a call asking none. */
    asking(request: Request): LoanAsk | undefined {
        // ask nothing without the header
        const header = request.headers.get(Loan.asked);
        if (header === null) {
            return undefined;
        }

        // read the method and the delegation, refusing a malformed ask
        const [method, delegation] = header.split(" ");
        const asked = LoanAsk.safeParse({ method, delegation });
        if (!asked.success) {
            throw new ServiceError("BAD_REQUEST", { message: `invalid ${Loan.asked}` });
        }

        return asked.data;
    },

    /** Sign a Loan as the home with its active signing key, lasting a minute. */
    async sign(
        sign: (payload: JWTPayload) => Promise<string>,
        grant: LoanGrant,
        now: number,
    ): Promise<string> {
        const issuedAt = Math.floor(now / 1000);

        return sign(
            LoanClaims.parse({
                iss: grant.home,
                sub: grant.person,
                aud: grant.space,
                lxm: grant.method,
                ...(grant.actor === undefined ? {} : { act: { sub: grant.actor } }),
                iat: issuedAt,
                exp: issuedAt + LOAN_MILLISECONDS / 1000,
                jti: crypto.randomUUID(),
            }),
        );
    },

    /** Verify a Loan for this space against its home's published keys, answering the person lent to the acting installation for the one method. */
    async verify(token: string, options: LoanVerification, now: number): Promise<Caller> {
        // verify the signature of the home it claims, for this space, within a minute
        const issuer = schema.identifier("space").safeParse(Bearer.issuer(token));
        if (!issuer.success) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid loan" });
        }
        let claims: schema.Infer<typeof LoanClaims>;
        try {
            const { payload } = await jwtVerify(token, options.keys(issuer.data), {
                issuer: issuer.data,
                audience: options.space,
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "iat", "exp", "jti"],
                maxTokenAge: LOAN_MILLISECONDS / 1000,
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                currentDate: new Date(now),
            });
            claims = LoanClaims.parse(payload);
        } catch (error) {
            throw error instanceof schema.Error
                ? new ServiceError("UNAUTHORIZED", { message: "invalid loan", cause: error })
                : Bearer.refusal(error, "invalid loan");
        }

        // require the person's home to have lent it
        const person = principal.user.reference(Scope.universe.id, claims.sub);
        if ((await options.home(person)) !== claims.iss) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "only a person's home lends their authority to another space",
            });
        }

        // answer the person, lent to the home's installation acting for them
        const actor =
            claims.act === undefined
                ? undefined
                : principal.installation.reference(claims.iss, claims.act.sub);

        return {
            subject: person,
            subjects: [person],
            ...(actor === undefined ? {} : { delegates: [{ subject: actor, authority: "lent" }] }),
            credential: {
                kind: "loan",
                id: claims.jti,
                audience: options.audience,
                scope: claims.aud,
                verifiedAt: now,
                expiresAt: claims.exp * 1000,
            },
            loan: { id: claims.jti, issuer: claims.iss, method: claims.lxm },
        };
    },

    /** Refuse a call of another method than the one a Loan lends, letting every other caller through. */
    require(caller: Pick<Caller, "loan">, method: string): void {
        const lent = caller.loan?.method;
        if (lent !== undefined && lent !== method) {
            throw new ServiceError("FORBIDDEN", {
                message: `the loan lends ${lent}, not ${method}`,
            });
        }
    },
};
