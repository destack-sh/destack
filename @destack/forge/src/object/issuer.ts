import { Issuer } from "@destack/connection/object";
import { defineObject } from "@destack/object";

/** The GitHub App's issuers, minting its installations' tokens for the connections they relate. */
export const issuer = defineObject(Issuer.definition(), Issuer.methods);
