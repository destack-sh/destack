import { Language } from "../../language/index.ts";
import { javascriptParser } from "../javascript/javascript.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of TypeScript. */
export default new LezerGrammar(Language.of("typescript"), javascriptParser.ts);
