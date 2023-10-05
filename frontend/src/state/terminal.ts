import { QueryOp, SortOrder, type Run, RunStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { SessionAccessLevel, useCurrentSessions, useRuns } from "@/state/session";
import { createSharedComposable } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef } from "vue";
import { getTypedKey } from "@/";

export type TerminalRun = {
  text?: string;
  code: string;
  run: Run;
};
export const TERMINAL_BOT_LABEL = "symbolx.bench.terminal";
const TERMINAL_TEXT_TO_CODE_TASK = "6d6ae26f-959b-4482-a0a3-74b88577701d"; // nocheckin: use task from std (maybe configurable for debugging)

function _useTerminal() {
  const session = useCurrentSessions();
  const bench = useBenchState();
  const module = useCurrentModule();
  const botLabelKey = computed(() => module.runMetadataKey("bot"));
  const codeKey = computed(() => module.runMetadataKey("code"));
  const textToCodeTask = computed(() => module.statementOf(TERMINAL_TEXT_TO_CODE_TASK));
  const codeOutputKey = computed(() => {
    const field = textToCodeTask.value?.fields?.find((f) => f.name == "code");
    if (field == null) return null;
    return module.getTypedKey(field);
  });

  const { runs, loading, totalCount } = useRuns(
    {
      projectId: toRef(bench, "projectId"),
      projectVersionId: toRef(bench, "projectVersionId"),
      statementIds: ref(null),
      statementCks: ref(null),
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
          // past 72h
          {
            op: QueryOp.GreaterThan,
            key: "created_at",
            value: DateTime.local().minus({ days: 3 }).toISO(),
          },
        ],
      })),
      sort: ref([{ key: "created_at", order: SortOrder.Descending }]),
      after: ref(null),
    },
    {
      limit: 100,
      count: true,
      live: true,
      insertAt: "start",
      queryAsFilter: (run) => run.value?.[botLabelKey.value ?? ""] == TERMINAL_BOT_LABEL,
    }
  );
  const terminalRuns = computed(() =>
    runs.value?.map((r) => ({
      code: r.value[codeKey.value ?? ""] ?? r.value["code"],
      run: r,
    }))
  );

  async function runText(
    text: string,
    options: { scope?: string; runMode: "approve" | "immediate"; accessLevel?: SessionAccessLevel } = {
      runMode: "approve",
    }
  ): Promise<{ code: string; run?: Run }> {
    if (textToCodeTask.value == null) {
      throw new Error("text to code task not found");
    }
    const { result } = session.run(textToCodeTask.value, {
      inputs: { text },
      globalValue: { bot: TERMINAL_BOT_LABEL },
      accessLevel: SessionAccessLevel.Read,
      keyed: false,
      block: 60,
      timeoutSeconds: 60,
    });
    const { run } = await result;
    const code = run?.outputs?.[codeOutputKey.value ?? ""];
    if (run?.status != RunStatus.Completed || code == null) {
      console.warn("run failed", run);
      throw new Error(`run failed: ${run?.errorNice?.kind} ${run?.errorNice?.message}`);
    }
    if (options?.runMode == "immediate") {
      throw new Error("immediate run mode not supported yet?");
    }
    return { code };
  }

  function runCode(code: string, options: { scope?: string; accessLevel: SessionAccessLevel }) {
    return session.run(code, {
      scope: options.scope,
      rootValue: { name: "terminal", bot: TERMINAL_BOT_LABEL, code, scope: options.scope },
      globalValue: { bot: TERMINAL_BOT_LABEL },
      accessLevel: options.accessLevel,
    });
  }

  return {
    runs: terminalRuns,
    loading,
    totalCount,
    runText,
    runCode,
  };
}

export const useTerminal = createSharedComposable(_useTerminal);
