import { DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";

/** The part of a secret's description stack reads: its name, beside what its store adds. */
const SecretEnvelope = schema.looseObject({
    /** The package-local secret name. */
    name: DeclarationName,
});

/** Describe a declared secret by its name, keeping what its store describes beside it. */
export function describeSecret(secret: {
    readonly name: string;
}): schema.Infer<typeof SecretEnvelope> {
    return SecretEnvelope.parse(secret);
}
