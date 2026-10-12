import { type SecretDeclaration, SecretDescription } from "../declare/secret.ts";

/** Describe a declared secret for the package manifest, with how a space generates it. */
export function describeSecret(secret: SecretDeclaration): SecretDescription {
    return SecretDescription.parse({
        name: secret.name,
        ...(secret.generated === undefined ? {} : { generated: secret.generated }),
    });
}
