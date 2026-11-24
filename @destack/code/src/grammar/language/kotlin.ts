import { kotlin } from "@codemirror/legacy-modes/mode/clike";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Kotlin. */
export default new StreamGrammar(Language.of("kotlin"), kotlin);
