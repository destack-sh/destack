<script lang="ts" setup>
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import { renderSimpleType } from "@/components/statement";
import { useNow, useTimeFromNow } from "@/composables/useNow";
import { StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
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
const ops = useOperations();
const notifications = useNotifications();
const editor = useEditorState();

async function run() {
  if (symbol.value == null) {
    return;
  }
  console.log("run " + symbol.value?.name, arguments_.value);
  const ret = await ops.runtime.run(symbol.value.id, build.value?.id, arguments_.value);
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
}

// TODO @Broken: get proper runnable id(s) if this is a not a code symbol
const { executions } = useExecutions(toRef(editor, "currentProjectVersionId"), toRef(props, "runnableId"), false);

const { getTimeFromNowString } = useTimeFromNow();
</script>
<template>
  <div
    class="mx-auto flex max-w-[1000px] flex-col items-baseline bg-white px-12 py-8"
    :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
  >
    <!-- Header -->
    <div class="mx-auto w-full max-w-[1000px]">
      <!-- Runnable -->
      <div class="flex flex-row gap-1">
        <button class="rounded-sm text-orange-600 outline-none hover:bg-orange-50" @click="run">run</button>
        <span>{{ symbol?.name ?? "???" }}</span>
        <!-- Build -->
        <template v-if="symbol?.symbolType == SymbolType.Task">
          <span class="text-orange-600">with</span>
          <ReferenceComboCell
            :reference="build"
            @set-reference="setBuild($event ?? undefined)"
            :available-symbols="availableBuilds"
          />
        </template>
        <button
          class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
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
    <!-- Outputs -->
    <div
      class="mt-6 grid gap-x-3 gap-y-3"
      :style="{ 'grid-template-columns': `repeat(${inputFields.length + 3}, minmax(40px, 100px))` }"
    >
      <template v-for="execution in executions" :key="execution.id">
        <!-- Execution status -->
        <div class="flex flex-row gap-1">
          {{ execution.status }}
          <span class="text-gray-500">{{ getTimeFromNowString(execution.updatedAt) }}</span>
        </div>
        <div>
          {{ execution.terminatedAt }}
        </div>
        <!-- Inputs -->
        <div v-for="field in inputFields" :key="field.id">
          <InlineValueCell
            :type="field"
            :model-value="execution.inputs?.[field.name]"
            :readonly="true"
            :immediate="false"
          />
        </div>
        <!-- Outputs -->
        <div>
          <InlineValueCell
            v-if="outputField"
            :type="outputField"
            :model-value="execution.outputs"
            :readonly="true"
            :immediate="false"
          />
        </div>
      </template>
    </div>
  </div>
</template>
