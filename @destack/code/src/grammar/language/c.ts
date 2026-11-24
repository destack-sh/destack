import { parser } from "@lezer/cpp";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of C. */
export default new LezerGrammar(Language.of("c"), parser);
