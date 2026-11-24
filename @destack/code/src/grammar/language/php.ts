import { parser } from "@lezer/php";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of PHP. */
export default new LezerGrammar(Language.of("php"), parser);
