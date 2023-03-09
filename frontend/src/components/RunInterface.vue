<script lang="ts" setup>
import RunsTable from "@/components/basic/RunsTable.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import { renderSimpleType } from "@/components/statement";
import { formatDiffSeconds, humanizeNumber, useTimeFromNow } from "@/composables/useNow";
import { ExecutionStatus, ExecutionTriggerType, StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { EDITOR_INTERFACE_STATE, useEditorState, type EditorInterfaceState } from "@/state/editor";
import { useExecutions } from "@/state/executions";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { symbolOf, symbolsLike } from "@/state/runtime";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { computed, inject, ref, toRef, type Ref } from "vue";

const props = defineProps<{ runnableId: string; runnableType: SymbolType }>();

const symbol = computed(() => symbolOf(props.runnableId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((n) => !n.isOutput) ?? []);
const outputField = computed(() => symbol.value?.typeNodes?.find((n) => n.isOutput));
const availableBuilds = symbolsLike({ types: [StatementType.Definition], symbolTypes: [SymbolType.Build] });

// local run interface state
const state = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
if (state == null) {
  throw new Error("need interface state context");
}

const build: Ref<InterpSymbol | undefined> = computed(() => symbolOf(state.get("buildId", "")));
function setBuild(build?: InterpSymbol) {
  state?.set("buildId", build?.id);
}
const arguments_: Ref<Record<string, any>> = computed(() => state.get("arguments", {}) as Record<string, any>);
function setArgument(key: string, value: string) {
  const args = { ...arguments_.value };
  args[key] = value;
  state?.set("arguments", args);
}

const lastOutput: Ref<any | null> = ref(null);
const lastOutputDirty = ref(false);
const ops = useOperations();
const notifications = useNotifications();
const editor = useEditorState();
const appearance = useAppearance();

async function run() {
  if (symbol.value == null) {
    return;
  }
  // prune arguments to only those that are defined
  const args = Object.fromEntries(
    Object.entries(arguments_.value).filter(([key]) => inputFields.value.some((f) => f.name == key))
  );
  console.log("run " + symbol.value?.name, args);
  lastOutputDirty.value = true;
  const ret = await ops.runtime.run(symbol.value.id, build.value?.id, args);
  if (ret?.errors || ret?.data?.run.__typename != "RunState" || !ret?.data?.run.success) {
    notifications.show({
      type: "run.fail",
      kind: "error",
      message: "Run failed",
      description: `Failed to run ${symbol.value?.name}.`,
    });
    lastOutput.value = null;
  } else {
    lastOutput.value = ret.data.run.output;
  }
  lastOutputDirty.value = false;
}

function openRunsEditor() {
  const e = editor.openRuns({ create: true });
  editor.focusEditor(e);
}

const includeAncestorVersions = ref(true);

const runsTableRef = ref<InstanceType<typeof RunsTable> | null>(null);
const inputColumns = computed(() =>
  inputFields.value == null ? undefined : inputFields.value.map((f) => [f.name, f])
);
const outputColumns = computed(() => (outputField.value == null ? undefined : [["Output", outputField.value]]));
</script>
<template>
  <div
    class="flex flex-col items-baseline bg-white px-12 py-8"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <!-- Runconfig -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="flex flex-row items-baseline gap-1">
        <span class="text-xl font-bold text-gray-900">Run</span>
      </h2>
      <!-- Runnable (supposed to imitate corresponding statement look) -->
      <div class="mt-2 flex flex-row gap-1">
        <button class="rounded-sm text-orange-600 outline-none hover:bg-orange-50" @click="run">run</button>
        <!-- TODO @Feature: should really be able to change the runnable inside Run interface -->
        <span>{{ symbol?.name ?? "???" }}</span>
        <!-- Build -->
        <template v-if="symbol?.symbolType == SymbolType.Task">
          <span class="text-orange-600">on</span>
          <ReferenceComboCell
            :reference="build"
            @set-reference="setBuild($event ?? undefined)"
            :available-symbols="availableBuilds"
          />
        </template>
        <button
          class="w-fit rounded-sm px-0.5 font-semibold text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
          @click="run"
        >
          <PlayIcon class="h-4 w-4 text-orange-600" />
        </button>
      </div>
      <!-- Arguments -->
      <div class="grid-w-fit my-1 grid grid-cols-[minmax(40px,auto)_1fr] gap-x-4">
        <template v-for="field in inputFields" :key="field.id">
          <div class="flex flex-row gap-1">
            <span>{{ field.name }}</span>
            <span class="text-gray-400">{{ renderSimpleType(field) }}</span>
          </div>
          <InlineValueCell
            :model-value="arguments_[field.name as string]"
            @update:model-value="(val: any) => setArgument(field.name as string, val)"
            :type="field"
            :readonly="false"
            immediate
          />
        </template>
      </div>
    </div>
    <!-- Current/last output  -->
    <div class="relative mx-auto mt-6 min-h-[100px] w-full max-w-[800px] border border-orange-900 border-opacity-[12%]">
      <span class="absolute -top-4 left-1 bg-white p-1 text-gray-700">Last output</span>
      <div class="animate-none px-2" v-if="lastOutput">
        <InlineValueCell
          v-if="outputField"
          :type="outputField"
          :model-value="lastOutput"
          :readonly="true"
          :immediate="false"
        />
      </div>
    </div>
    <!-- Runs -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="mt-6 flex flex-row items-baseline gap-1">
        <button
          class="text-xl font-bold text-gray-900 decoration-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="openRunsEditor"
        >
          Runs
        </button>
        <span class="rounded-3xl bg-gray-100 py-0.5 px-1 text-sm text-gray-900" v-if="runsTableRef">
          {{ humanizeNumber(runsTableRef?.totalCount) }}
        </span>
      </h2>
      <RunsTable
        class="mt-2"
        ref="runsTableRef"
        :project-id="editor.currentProjectId"
        :project-version-id="editor.currentProjectVersionId"
        :include-ancestor-versions="includeAncestorVersions"
        :build-ids="build ? [build.id] : undefined"
        :task-ids="runnableType == SymbolType.Task ? [runnableId] : undefined"
        :code-ids="runnableType == SymbolType.Code ? [runnableId] : undefined"
        :input-columns="inputColumns"
        :output-columns="outputColumns"
        override-from-props
      />
    </div>
  </div>
</template>
