import { writeFile } from "node:fs/promises";
import { Source } from "./source.ts";
import { TOKENS } from "./token.ts";

/** The package directory. */
const ROOT = new URL("../../", import.meta.url);

// write the StyleX constants and the text styles
await writeFile(new URL("src/token/tokens.stylex.ts", ROOT), Source.constants(TOKENS));
await writeFile(new URL("src/text/text.ts", ROOT), Source.text(TOKENS));
