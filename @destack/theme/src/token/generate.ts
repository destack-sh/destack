import { fileURLToPath } from "node:url";
import { BunEnvironment } from "@destack/environment/bun";
import { Source } from "./source.ts";
import { TOKENS } from "./token.ts";

/** The package directory. */
const ROOT = new URL("../../", import.meta.url);

/** The files of the machine the generator runs on. */
const { files } = BunEnvironment.system;

// write the StyleX constants and the text styles
await files.writeTextFile(
    fileURLToPath(new URL("src/token/tokens.stylex.ts", ROOT)),
    Source.constants(TOKENS),
);
await files.writeTextFile(fileURLToPath(new URL("src/text/text.ts", ROOT)), Source.text(TOKENS));
