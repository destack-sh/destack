import { Language } from "../../language/index.ts";
import { javascriptParser } from "../javascript/javascript.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of TSX. */
export default new LezerGrammar(Language.of("tsx"), javascriptParser.tsx);
