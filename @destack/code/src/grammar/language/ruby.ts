import { ruby } from "@codemirror/legacy-modes/mode/ruby";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Ruby. */
export default new StreamGrammar(Language.of("ruby"), ruby);
