import { parser } from "@lezer/xml";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of XML. */
export default new LezerGrammar(Language.of("xml"), parser);
