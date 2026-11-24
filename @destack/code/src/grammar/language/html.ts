import { Language } from "../../language/index.ts";
import { htmlParser } from "../html.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of HTML. */
export default new LezerGrammar(Language.of("html"), htmlParser);
