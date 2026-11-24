import { properties } from "@codemirror/legacy-modes/mode/properties";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of INI. */
export default new StreamGrammar(Language.of("ini"), properties);
