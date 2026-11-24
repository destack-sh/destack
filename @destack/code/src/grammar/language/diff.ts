import { diff } from "@codemirror/legacy-modes/mode/diff";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Diff. */
export default new StreamGrammar(Language.of("diff"), diff);
