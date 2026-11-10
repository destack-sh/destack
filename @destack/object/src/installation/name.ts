import { DeclarationName } from "@destack/package";

/** The longest installation name, sized with an account handle to fit one DNS label. */
const INSTALLATION_NAME_LENGTH = 15;

/** The texts other than `xn`, which starts punycode labels, as a pattern without lookarounds. */
export const UNPREFIXED_LABEL = /^(?:$|[^x]|x(?:$|[^n])|xn[\s\S])/u;

/** An installation's name within its space, unique there and fixed once installed. */
export const InstallationName =
    DeclarationName.max(INSTALLATION_NAME_LENGTH).regex(UNPREFIXED_LABEL);
