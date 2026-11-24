import { parser } from "@lezer/sass";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of SCSS. */
export default new LezerGrammar(Language.of("scss"), parser);
