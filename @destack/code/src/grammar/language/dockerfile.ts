import { dockerFile } from "@codemirror/legacy-modes/mode/dockerfile";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of Dockerfile. */
export default new StreamGrammar(Language.of("dockerfile"), dockerFile);
