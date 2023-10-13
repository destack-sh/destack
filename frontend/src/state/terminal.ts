import { QueryOp, SortOrder, type Run, RunStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { SessionAccessLevel, useCurrentSessions, useRuns } from "@/state/session";
import { createSharedComposable } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef } from "vue";

export type TerminalRun = {
  text?: string;
  code: string;
  run: Run;
};
export const TERMINAL_BOT_LABEL = "symbolx.bench.terminal";
export const STDLIB_TEXT_TO_CODE_TASK_CK = "0c8e5c97-7433-51e6-8d2f-61893fc48d04";

function _useTerminal() {
  const session = useCurrentSessions();
  const bench = useBenchState();
  const module = useCurrentModule();
  const botLabelKey = computed(() => module.runMetadataKey("bot"));
  const codeKey = computed(() => module.runMetadataKey("code"));
  const scopeKey = computed(() => module.runMetadataKey("scope"));
  const textToCodeTask = computed(() => module.statementOf("0c8e5c97-7433-51e6-8d2f-61893fc48d04"));
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
      statementIds: ref([]),
      statementCks: ref([]),
      rootOnly: ref(true),
      sessionId: ref(null),
      runId: ref(null),
      query: computed(() => ({
        op: QueryOp.And,
        queries: [
          {
            op: QueryOp.Equals,
            key: "value." + botLabelKey.value,
            value: TERMINAL_BOT_LABEL,
          },
          // past week
          {
            op: QueryOp.GreaterThan,
            key: "created_at",
            value: DateTime.local().minus({ days: 7 }).toISO(),
          },
        ],
      })),
      sort: ref([{ key: "created_at", order: SortOrder.Descending }]),
      after: ref(null),
    },
    {
      limit: 128,
      count: true,
      live: true,
      insertAt: "start",
      queryAsFilter: (run) => run.value?.[botLabelKey.value ?? ""] == TERMINAL_BOT_LABEL && run.statement == null,
    }
  );
  const terminalRuns = computed(() =>
    runs.value?.map((r) => ({
      code: r.value[codeKey.value ?? ""] ?? r.value["code"],
      scope: r.value[scopeKey.value ?? ""] ?? r.value["scope"],
      generatedFrom: r.value[generatedFromKey.value ?? ""] ?? r.value["generated_from"],
      generatedIn: r.value[generatedInKey.value ?? ""] ?? r.value["generated_in"],
      run: r,
    }))
  );

  function runTextToCode(text: string): { result: Promise<{ code: string }>; run: Run } {
    /** Converts the given rich text (with mentions) to Bench code */
    if (textToCodeTask.value == null) {
      throw new Error("text to code task not found");
    }
    const { run, result } = session.run(textToCodeTask.value, {
      inputs: { text },
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
      cleanCode?: string;
    }
  ) {
    /** Runs code inside the terminal, raising if the run fails to complete */
    const { run, result } = session.run(code, {
      scope: options.scope,
      rootValue: {
        name: "terminal",
        code: options.cleanCode ?? code,
        scope: options.scope,
        generated_from: options.generatedFrom,
        generated_in: options.generatedIn,
      },
      globalValue: { bot: TERMINAL_BOT_LABEL },
      accessLevel: options.accessLevel,
      tags: options.tags,
    });
    return {
      run,
      result: result.then(({ run, logs }) => {
        if (run.status != RunStatus.Completed) {
          throw new Error(`run failed: ${run?.errorNice?.kind} ${run?.errorNice?.message}`);
        }
        return { run, logs };
      }),
    };
  }

  return {
    runs: terminalRuns,
    loading,
    totalCount,
    runTextToCode,
    runCode,
  };
}

export const useTerminal = createSharedComposable(_useTerminal);
