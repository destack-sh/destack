import type { Subject } from "@destack/sync";

/** An external service accounts connect to. */
export interface ConnectionProvider {
    /** The provider's name connections record, such as github. */
    readonly name: string;
    /** The provider instance, such as https://github.com or an enterprise installation. */
    readonly issuer: string;
    /** The provider application receiving authorizations, such as its OAuth client identifier. */
    readonly applicationId: string;
    /** Whether the provider grants a user an OAuth credential or installs the application. */
    readonly kind: "oauth" | "installation";
    /** Build the provider page where the user authorizes the application. */
    authorize(request: ConnectionRequest): URL;
    /** Exchange the provider's callback parameters for the authorized external account. */
    complete(callback: ConnectionCallback): Promise<ConnectionGrant>;
    /** Withdraw the application's authorization at the provider. */
    revoke(grant: ConnectionRevocation): Promise<void>;
}

/** What the provider's authorization page carries back to the account service. */
export interface ConnectionRequest {
    /** The state the provider returns with its callback. */
    readonly state: string;
    /** The PKCE S256 code challenge of the pending authorization's verifier. */
    readonly challenge: string;
    /** The provider scopes requested. */
    readonly scopes: readonly string[];
}

/** The provider's callback to a pending authorization. */
export interface ConnectionCallback {
    /** The provider's callback parameters, such as its code. */
    readonly parameters: Readonly<Record<string, string>>;
    /** The PKCE code verifier the code exchange presents. */
    readonly verifier: string;
    /** The provider scopes requested. */
    readonly scopes: readonly string[];
}

/** The external account a provider authorized. */
export interface ConnectionGrant {
    /** The provider's stable account identifier. */
    readonly subject: string;
    /** The provider scopes granted. */
    readonly scopes: readonly string[];
    /** Provider-specific permission levels granted to the application. */
    readonly permissions: Readonly<Record<string, string>>;
    /** The provider's installation identifier, for installations. */
    readonly installationId?: string;
    /** The credential an OAuth grant keeps in the vault, such as its serialized tokens. */
    readonly credential?: string;
    /** The credential expiry, when the provider sets one. */
    readonly expiresAt?: number;
}

/** An authorization the account service withdraws at its provider. */
export interface ConnectionRevocation {
    /** The provider's stable account identifier. */
    readonly subject: string;
    /** The provider's installation identifier, for installations. */
    readonly installationId?: string;
    /** The credential an OAuth grant keeps in the vault. */
    readonly credential?: string;
}

/** The vault keeping connections' OAuth credentials as secrets. */
export interface Vault {
    /** Keep a credential as a secret. */
    write(secret: {
        readonly id: string;
        readonly spaceId: string;
        readonly vaultId: string;
        readonly name: string;
        readonly value: string;
        readonly subject: Subject;
    }): Promise<void>;
    /** Read the current value of a secret. */
    read(secret: {
        readonly spaceId: string;
        readonly secretId: string;
        readonly subject: Subject;
    }): Promise<string>;
    /** Destroy a secret. */
    destroy(secret: {
        readonly spaceId: string;
        readonly secretId: string;
        readonly subject?: Subject;
    }): Promise<void>;
}
