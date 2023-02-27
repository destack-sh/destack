<script lang="ts" setup>
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import { renderSimpleType } from "@/components/statement";
import { formatDiffSeconds, humanizeNumber, useNow, useTimeFromNow } from "@/composables/useNow";
import { ExecutionStatus, StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
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

// TODO @Broken: get proper runnable id(s) if this is a not a code symbol
const { executions, totalCount } = useExecutions(
  toRef(editor, "currentProjectVersionId") as Ref<string>,
  computed(() => state.get("buildId", null)),
  computed(() => (props.runnableType == SymbolType.Task ? props.runnableId : null)),
  computed(() => (props.runnableType == SymbolType.Code ? props.runnableId : null)),
  { root: true, live: true }
);

const { getTimeFromNowString, now } = useTimeFromNow(33);
</script>
<template>
  <div
    class="mx-auto flex max-w-[1000px] flex-col items-baseline bg-white px-12 py-8"
    :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
  >
    <!-- Runconfig -->
    <div class="mx-auto w-full max-w-[1000px]">
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
      <div class="grid-w-fit my-1 grid grid-cols-[minmax(40px,auto)_10px_1fr] gap-x-2">
        <template v-for="field in inputFields" :key="field.id">
          <div class="flex flex-row gap-1">
            <span>{{ field.name }}</span>
            <span class="text-gray-400">{{ renderSimpleType(field) }}</span>
          </div>
          <span>=</span>
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
    <div class="relative mt-6 min-h-[100px] w-full border border-gray-200">
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
    <h2 class="mt-6 flex flex-row items-baseline gap-1">
      <span class="text-xl font-bold text-gray-900">Runs</span>
      <span class="rounded-3xl bg-gray-100 py-0.5 px-1 text-sm text-gray-900">{{ humanizeNumber(totalCount) }}</span>
    </h2>
    <table
      class="mt-2 items-baseline divide-y-2 divide-gray-300/25"
      :style="{ 'grid-template-columns': `repeat(${inputFields.length + 3}, minmax(40px, 100px))` }"
    >
      <!-- Header -->
      <thead>
        <tr class="text-left">
          <th class="px-2 font-semibold text-gray-700">Status</th>
          <th class="px-2 font-semibold text-gray-700">Duration</th>
          <th v-for="field in inputFields" :key="field.id" class="px-2 font-semibold text-gray-700">
            {{ field.name }}
          </th>
          <th class="px-2 font-semibold text-gray-700">{{ outputField?.name ?? "Output" }}</th>
        </tr>
      </thead>
      <!-- Content -->
      <tbody>
        <tr v-for="execution in executions" :key="execution.id">
          <!-- Execution status -->
          <td class="flex flex-row items-center gap-1 px-2 py-2">
            <svg
              viewBox="0 0 100 100"
              class="h-3 w-3"
              :class="{
                'text-green-600': execution.status == ExecutionStatus.Completed,
                'text-red-600':
                  execution.status == ExecutionStatus.Failed || execution.status == ExecutionStatus.Aborted,
                'animate-pulse text-gray-500':
                  execution.status == ExecutionStatus.Created ||
                  execution.status == ExecutionStatus.Scheduled ||
                  execution.status == ExecutionStatus.Running,
              }"
            >
              <circle cx="50" cy="50" r="40" fill="currentColor" />
            </svg>
            <span class="text-gray-500">{{ getTimeFromNowString(execution.updatedAt) }}</span>
          </td>
          <td class="px-2 text-right text-gray-700">
            <span class="" v-if="execution.terminatedAt != null">
              {{ formatDiffSeconds(execution.startedAt, execution.terminatedAt) }}
            </span>
            <span v-else-if="execution.startedAt != null">
              {{ formatDiffSeconds(execution.startedAt, now) }}
            </span>
          </td>
          <!-- Inputs -->
          <td v-for="field in inputFields" :key="field.id" class="px-2">
            <InlineValueCell
              :type="field"
              :model-value="execution.inputs?.[field.name]"
              :readonly="true"
              :immediate="false"
            />
          </td>
          <!-- Outputs -->
          <td class="px-2">
            <InlineValueCell
              v-if="outputField"
              :type="outputField"
              :model-value="execution.outputs"
              :readonly="true"
              :immediate="false"
            />
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
