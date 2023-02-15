<script lang="ts" setup>
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { renderSimpleType } from "@/components/statement";
import { StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
import { EDITOR_INTERFACE_STATE, type EditorInterfaceState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { symbolOf, symbolsLike } from "@/state/runtime";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { computed, inject, type Ref } from "vue";

const props = defineProps<{ runnableId: string; runnableType: SymbolType }>();

const symbol = computed(() => symbolOf(props.runnableId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((n) => !n.isOutput) ?? []);
const availableBuilds = symbolsLike({ types: [StatementType.Definition], symbolTypes: [SymbolType.Build] });

// local run interface state
const state = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
if (state == null) {
  throw new Error("need interface state context");
}

const build: Ref<InterpSymbol | undefined> = computed(() => symbolOf(state.get("buildId", "")));
function setBuild(build: InterpSymbol) {
  state?.set("buildId", build.id);
}
const arguments_: Ref<Record<string, any>> = computed(() => state.get("arguments", {}) as Record<string, any>);
function setArgument(key: string, value: string) {
  const args = { ...arguments_.value };
  args[key] = value;
  state?.set("arguments", args);
}

const ops = useOperations();
async function run() {
  if (symbol.value == null) {
    return;
  }
  console.log("run " + symbol.value?.name, arguments_.value);

  let buildId = undefined; // TODO @Incomplete: select buildid for tasks
  if (symbol.value.symbolType == SymbolType.Task) {
    throw new Error("selecting build for tasks is not implemented yet");
  }
  await ops.runtime.run(symbol.value.id, buildId, arguments_.value);
}
</script>
<template>
  <div class="flex flex-col bg-white px-12 py-8 font-mono text-sm">
    <!-- Header -->
    <div class="flex flex-row gap-1">
      <button class="rounded-sm text-orange-600 outline-none hover:bg-orange-50" @click="run">run</button>
      <span>{{ symbol?.name ?? "???" }}</span>
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
    <!-- Output -->
    <!-- TODO @Inconmplete: show previous executions -->
  </div>
</template>
