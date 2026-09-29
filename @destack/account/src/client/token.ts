import type { PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { connect } from "./client.ts";

/** Renew shortly before the access token expires, measured in milliseconds. */
const RENEWAL_MARGIN_MILLISECONDS = 5000;

/** Retain one receiving-service token and renew it through the authenticated account client. */
export class AccountToken {
    /** Client retaining the primary session or API credential at the account service. */
    readonly client: ReturnType<typeof connect>;
    /** Fixed service and space selected by the application. */
    readonly selection: {
        /** Receiving service package. */
        readonly audience: PackageId;
        /** Exact space containing the protected records. */
        readonly spaceId: Identifier<"space">;
    };
    /** Most recently issued token, retained only in memory. */
    #token?: Awaited<ReturnType<ReturnType<typeof connect>["authentication"]["exchange"]>>;
    /** Shared renewal for concurrent client calls. */
    #pending?: ReturnType<ReturnType<typeof connect>["authentication"]["exchange"]>;

    /** Select the audience and space once, independently of downstream request inputs. */
    constructor(client: ReturnType<typeof connect>, selection: AccountToken["selection"]) {
        this.client = client;
        this.selection = { ...selection };
    }

    /** Return the current access token, coalescing concurrent renewal attempts. */
    async get(): Promise<string> {
        // use an unexpired token without contacting the global account service
        if (this.#token && this.#token.expiresAt > Date.now() + RENEWAL_MARGIN_MILLISECONDS) {
            return this.#token.accessToken;
        }

        // replace failed or expired credentials only through a successful exchange
        this.#pending ??= this.client.authentication.exchange(this.selection);
        const pending = this.#pending;
        try {
            const issued = await pending;
            if (issued.expiresAt <= Date.now()) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "received an expired access token",
                });
            }
            this.#token = issued;

            return issued.accessToken;
        } finally {
            if (this.#pending === pending) {
                this.#pending = undefined;
            }
        }
    }

    /** Provide authorization headers to any typed Destack service client. */
    async headers(): Promise<{ authorization: string }> {
        return { authorization: `Bearer ${await this.get()}` };
    }
}
