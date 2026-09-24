import type { Resource } from "@destack/resource";
import { VaultDescription } from "../declare/vault.ts";
import { SecretDescription, type defineSecret } from "../declare/secret.ts";

/** Describe a declared vault for the package manifest. */
export function describeVault(vault: Resource<unknown, VaultDescription>): VaultDescription {
    return VaultDescription.parse({
        name: vault.name,
        kind: vault.kind,
        version: vault.version,
        spec: vault.spec,
    });
}

/** Describe a declared secret for the package manifest. */
export function describeSecret(secret: ReturnType<typeof defineSecret>): SecretDescription {
    return SecretDescription.parse({ name: secret.name, version: secret.version });
}
