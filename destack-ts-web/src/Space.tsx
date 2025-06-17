import React, { useEffect, useRef } from "react";

import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, drawSelection, dropCursor, rectangularSelection } from "@codemirror/view";
import { python } from "@codemirror/lang-python";
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from "@codemirror/language";
import { defaultKeymap, historyKeymap } from "@codemirror/commands";
import { searchKeymap } from "@codemirror/search";
import { foldKeymap } from "@codemirror/language";
import { closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { lintKeymap } from "@codemirror/lint";

import { PyodideWorkerStatus, usePyodideWorker } from "./runtime/pyodide";
import { formatDuration } from "../../destack-ts/src/utils/time";

const Space: React.FC = () => {
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const editorViewRef = useRef<EditorView | null>(null);

  const { status, output, duration, isRunning, runCode } = usePyodideWorker();

  useEffect(() => {
    if (!editorContainerRef.current || editorViewRef.current) return; // already mounted

    const state = EditorState.create({
      doc: "# Python REPL\nprint('Hello, Python!')\n",
      extensions: [
        python(),
        lineNumbers(),
        dropCursor(),
        EditorView.lineWrapping,
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          ...searchKeymap,
          ...foldKeymap,
          ...closeBracketsKeymap,
          ...lintKeymap,
          ...completionKeymap,
        ]),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        bracketMatching(),
        rectangularSelection(),
        drawSelection(),
      ],
    });

    editorViewRef.current = new EditorView({ state, parent: editorContainerRef.current });

    return () => {
      editorViewRef.current?.destroy();
      if (editorContainerRef.current) editorContainerRef.current.innerHTML = ""; // remove leftover DOM
      editorViewRef.current = null;
    };
  }, []);

  const handleRunPython = () => {
    if (status !== PyodideWorkerStatus.READY || !editorViewRef.current) return;

    const code = editorViewRef.current.state.doc.toString();
    runCode(code);
  };

  return (
    <div className="p-4">
      <div className="mb-4">
        <div ref={editorContainerRef} className="border rounded min-h-[200px]" />
        <button
          onClick={handleRunPython}
          disabled={status !== PyodideWorkerStatus.READY || isRunning}
          className="mt-2 px-4 py-2 bg-blue-500 text-white rounded disabled:bg-gray-300"
        >
          {isRunning ? "Running..." : status === PyodideWorkerStatus.READY ? "Run" : status}
        </button>
      </div>

      {duration != null && (
        <span className="text-sm text-gray-600">
          Executed in {formatDuration(duration, { format: "short", minUnit: "ms" })}
        </span>
      )}
      {output && (
        <div className="mt-4">
          <div className="flex items-center justify-between mb-2">
            <h3 className="font-bold">Output:</h3>
          </div>
          <pre className="bg-gray-100 p-2 rounded overflow-auto">{output}</pre>
        </div>
      )}
    </div>
  );
};

export default Space;
