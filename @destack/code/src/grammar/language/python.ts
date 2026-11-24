import { parser } from "@lezer/python";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of Python. */
export default new LezerGrammar(Language.of("python"), parser);
