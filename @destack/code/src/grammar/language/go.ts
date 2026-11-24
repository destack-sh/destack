import { parser } from "@lezer/go";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of Go. */
export default new LezerGrammar(Language.of("go"), parser);
