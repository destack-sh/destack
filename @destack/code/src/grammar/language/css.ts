import { parser } from "@lezer/css";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of CSS. */
export default new LezerGrammar(Language.of("css"), parser);
