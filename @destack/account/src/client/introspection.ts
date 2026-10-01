import { Authentication } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import type { Identifier } from "@destack/schema";
import type { PackageId } from "@destack/package";
import type { connect } from "./client.ts";
import { Bearer } from "@destack/service/authentication";

/** Recheck credentials with AccountService when an operation requires current global state. */
export class AccountIntrospection {
    /** Authenticated account client. */
    readonly client: ReturnType<typeof connect>;
    /** Account selected for identity verification. */
    readonly accountId: Identifier<"account">;
    /** Receiving package identifier. */
    readonly audience: PackageId;

    /** Retain the host's privileged client and fixed receiving-service audience. */
    constructor(
        client: ReturnType<typeof connect>,
        accountId: Identifier<"account">,
        audience: PackageId,
    ) {
        this.client = client;
        this.accountId = accountId;
        this.audience = audience;
    }

    /** Verify a request's bearer credential and current memberships for the selected space. */
    async authenticate(request: Request, spaceId: Identifier<"space">): Promise<Authentication> {
        // require one bearer token without cookies
        const token = Bearer.read(request.headers);
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        // keep the receiving host's credential separate from the credential being verified
        const authentication = await this.client.authentication.verify(
            {
                accountId: this.accountId,
                audience: this.audience,
                spaceId,
                token,
            },
            { signal: request.signal },
        );

        // require the verified caller to hold for this audience and space now
        const caller = new Authentication(authentication);
        caller.context(this.audience, Date.now(), spaceId);

        return caller;
    }
}
