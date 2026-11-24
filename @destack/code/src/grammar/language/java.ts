import { parser } from "@lezer/java";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of Java. */
export default new LezerGrammar(Language.of("java"), parser);
