import type { Resource } from "@destack/resource";
import { VaultKind, type VaultDescription } from "../declare/vault.ts";
import { SecretDescription, type defineSecret } from "../declare/secret.ts";

/** Describe a declared vault for the package manifest. */
export function describeVault(vault: Resource<unknown, VaultDescription>): VaultDescription {
    return VaultKind.description.parse({ name: vault.name, kind: vault.kind, spec: vault.spec });
}

/** Describe a declared secret for the package manifest. */
export function describeSecret(secret: ReturnType<typeof defineSecret>): SecretDescription {
    return SecretDescription.parse({ name: secret.name });
}
