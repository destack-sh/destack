import { shell } from "@codemirror/legacy-modes/mode/shell";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Bash. */
export default new StreamGrammar(Language.of("bash"), shell);
