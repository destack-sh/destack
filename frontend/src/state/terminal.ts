import { ConditionalOp, SortOp, type Run, RunStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { SessionAccessLevel, useCurrentSessions, useRuns } from "@/state/session";
import { createSharedComposable } from "@vueuse/core";
import { computed, ref, toRef, type Ref, watchEffect } from "vue";

export type TerminalRun = {
  text?: string;
  code: string;
  run: Run;
};
export const TERMINAL_BOT_LABEL = "symbolx.bench.terminal";
export const STDLIB_TEXT_TO_CODE_TASK_CK = "9904403b-a09b-421f-a8b9-8ca2fa3cea9a";

function _useTerminal() {
  const session = useCurrentSessions();
  const bench = useBenchState();
  const module = useCurrentModule();
  const botLabelKey = computed(() => module.runMetadataKey("bot"));
  const codeKey = computed(() => module.runMetadataKey("code"));
  const scopeKey = computed(() => module.runMetadataKey("scope"));
  const textToCodeTask = computed(() => module.statementOf(STDLIB_TEXT_TO_CODE_TASK_CK));
  const generatedFromKey = computed(() => module.runMetadataKey("generated from"));
  const generatedInKey = computed(() => module.runMetadataKey("generated in"));
  const codeOutputKey = computed(() => {
    const field = textToCodeTask.value?.fields?.find((f) => f.name == "code");
    if (field == null) return null;
    return module.getTypedKey(field);
  });

  const { runs, loading, totalCount } = useRuns(
    {
      projectId: toRef(bench, "projectId"),
      projectVersionId: toRef(bench, "projectVersionId"),
      statementCks: ref([]),
      rootOnly: ref(true),
      sessionId: ref(null),
      runId: ref(null),
      query: computed(() => ({
        op: ConditionalOp.And,
        clauses: [
          {
            op: ConditionalOp.Equals,
            field: "value." + botLabelKey.value,
            value: TERMINAL_BOT_LABEL,
          },
        ],
      })),
      sort: ref([{ field: "created_at", order: SortOp.Descending }]),
      after: ref(null),
    },
    {
      limit: 64,
      count: true,
      live: true,
      insertAt: "start",
      queryAsFilter: (run) => run.value?.[botLabelKey.value ?? ""] == TERMINAL_BOT_LABEL && run.statementCk == null,
      neverUnsubscribe: true, // shared/immortal composable
    }
  );
  const pendingRuns: Ref<Run[]> = ref([]);
  const terminalRuns = computed(() => {
    const terminalRuns = [...(runs.value ?? [])];
    // add pending runs to the top of the list
    for (const run of pendingRuns.value) {
      //  (pending runs and terminal runs may overlap briefly)
      if (!terminalRuns.some((r) => r.id == run.id)) {
        terminalRuns.unshift(run);
      }
    }
    return terminalRuns.map((r) => ({
      code: r.value[codeKey.value ?? ""] ?? r.value["code"],
      scope: r.value[scopeKey.value ?? ""] ?? r.value["scope"],
      generatedFrom: r.value[generatedFromKey.value ?? ""] ?? r.value["generated_from"],
      generatedIn: r.value[generatedInKey.value ?? ""] ?? r.value["generated_in"],
      run: r,
    }));
  });

  function runTextToCode(
    text: string,
    config?: { nonce?: string; mode?: "fast" | "deliberate" }
  ): { result: Promise<{ code: string }>; run: Run } {
    /** Converts the given rich text (with mentions) to Bench code */
    if (textToCodeTask.value == null) {
      throw new Error("text to code task not found");
    }
    const { run, firstResult: result } = session.run(textToCodeTask.value, {
      inputs: { text, ...config },
      globalValue: { bot: TERMINAL_BOT_LABEL },
      accessLevel: SessionAccessLevel.Read,
      keyed: false,
      block: 60,
      timeoutSeconds: 60,
    });
    const codeResult = result.then(({ run, logs }) => {
      const code = run?.outputs?.[codeOutputKey.value ?? ""] as string;
      if (run?.status != RunStatus.Completed || code == null) {
        throw new Error(`run failed: ${run?.errorNice?.kind} ${run?.errorNice?.message}`);
      }
      return { code };
    });
    return { result: codeResult, run };
  }

  function runCode(
    code: string,
    options: {
      scope?: string;
      accessLevel: SessionAccessLevel;
      tags?: string[];
      generatedFrom?: string;
      generatedIn?: string;
    }
  ) {
    /** Runs code inside the terminal, raising if the run fails to complete */
    const { run, firstResult, finalResult } = session.run(code, {
      scope: options.scope,
      rootValue: {
        code: code,
        scope: options.scope,
        generated_from: options.generatedFrom,
        generated_in: options.generatedIn,
      },
      globalValue: { bot: TERMINAL_BOT_LABEL },
      accessLevel: options.accessLevel,
      tags: options.tags,
    });
    pendingRuns.value = [...pendingRuns.value, run];
    firstResult.then(() => {
      pendingRuns.value = pendingRuns.value.filter((r) => r.id != run.id);
    });
    return { run, firstResult, finalResult };
  }

  return {
    runs: terminalRuns,
    pendingRuns, // for debugging
    loading,
    totalCount,
    runTextToCode,
    runCode,
  };
}

export const useTerminal = createSharedComposable(_useTerminal);
