import { parser } from "@lezer/json";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of JSON. */
export default new LezerGrammar(Language.of("json"), parser);
