import { csharp } from "@codemirror/legacy-modes/mode/clike";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of C#. */
export default new StreamGrammar(Language.of("csharp"), csharp);
