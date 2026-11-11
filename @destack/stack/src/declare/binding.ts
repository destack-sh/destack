import { DeclarationName, defineKind } from "@destack/package";
import { schema } from "@destack/schema";

/** The binding target type of a secret of the space. */
export const SECRET_TARGET = "secret";

/** A declared secret as stack binds it: its name, leaving how vault generates it to vault. */
const SecretBinding = schema.looseObject({
    /** The package-local secret name. */
    name: DeclarationName,
});

/** The secret declaration kind, which stack defines and vault's secrets make. */
export const SECRET_KIND = defineKind({ name: "secret", description: SecretBinding });
