import { swift } from "@codemirror/legacy-modes/mode/swift";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Swift. */
export default new StreamGrammar(Language.of("swift"), swift);
