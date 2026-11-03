import { defineDeclarationKind } from "@destack/package";
/** The binding target type of a secret of the space. */
export const SECRET_TARGET = "secret";

/** The secret declaration kind, which stack defines and vault's secrets make. */
export const SECRET_KIND = defineDeclarationKind("secret");
