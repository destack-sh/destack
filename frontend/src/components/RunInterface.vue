<script lang="ts" setup>
import RunsTable from "@/components/basic/RunsTable.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import { renderSimpleType } from "@/components/statement";
import { humanizeNumber } from "@/composables/useNow";
import { StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { EDITOR_INTERFACE_STATE, useEditorState, type EditorInterfaceState } from "@/state/editor";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { symbolOf, symbolsLike } from "@/state/runtime";
import { QuestionMarkCircleIcon } from "@heroicons/vue/20/solid";
import { ArrowDownIcon } from "@heroicons/vue/24/outline";
import { computed, inject, ref, type Ref } from "vue";

const props = defineProps<{ runnableId: string; runnableType: SymbolType }>();

const symbol = computed(() => symbolOf(props.runnableId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((n) => !n.isOutput) ?? []);
const outputField = computed(() => symbol.value?.typeNodes?.find((n) => n.isOutput));
const availableBuilds = symbolsLike({ types: [StatementType.Definition], symbolTypes: [SymbolType.Build] });
const includeAncestorVersions = ref(true);

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
const runsTableRef = ref<InstanceType<typeof RunsTable> | null>(null);
const runButtonRef = ref<HTMLButtonElement | null>(null);
const selectBuildRef = ref<InstanceType<typeof ReferenceComboCell> | null>(null);
const argumentsGrid = useNavigationGrid<"value", InstanceType<typeof InlineValueCell>>(
  computed(() => ["value"]),
  inputFields,
  {
    gridNavigateUp: () => selectBuildRef.value?.focus(),
    gridNavigateDown: () => runButtonRef.value?.focus(),
  }
);
const lastOutput: Ref<any | null> = ref(null);
const lastOutputDirty = ref(false);
const ops = useOperations();
const notifications = useNotifications();
const actions = useActions();
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
      description: `Failed to run ${symbol.value?.name}: ${ret?.data?.run?.error ?? "rejected"}`,
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

const inputColumns = computed(() =>
  inputFields.value == null ? undefined : inputFields.value.map((f) => [f.name, f])
);
const outputColumns = computed(() => (outputField.value == null ? undefined : [["Output", outputField.value]]));
</script>
<template>
  <div
    class="flex flex-col items-baseline bg-white px-12 py-6"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <!-- Runconfig -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="flex flex-row items-baseline gap-1">
        <span class="text-3xl font-bold text-gray-900">Run {{ symbol?.name }}</span>
      </h2>
      <!-- Runnable (supposed to imitate corresponding statement look) -->
      <div class="mt-2 flex flex-row justify-between">
        <!-- Build select -->
        <span class="flex flex-row gap-1">
          <span class="text-gray-500">Build:</span>
          <template v-if="symbol?.symbolType == SymbolType.Task">
            <ReferenceComboCell
              ref="selectBuildRef"
              :reference="build"
              @set-reference="setBuild($event ?? undefined)"
              :available-symbols="availableBuilds"
              @navigate-down="argumentsGrid.focus(0, 'value')"
            />
          </template>
        </span>
        <!-- Deploy link/help -->
        <span class="flex flex-row items-center gap-0.5">
          <button
            class="rounded-sm px-1 text-gray-700 hover:bg-orange-100 hover:text-gray-900"
            @click="actions.apply('version.deploy')"
          >
            Deploy
          </button>
          <router-link
            to="/symbolx/docs#Deploying"
            class="text-gray-400 hover:bg-orange-100 hover:text-gray-900"
            target="_blank"
          >
            <QuestionMarkCircleIcon class="h-4 w-4" />
          </router-link>
        </span>
      </div>
      <!-- Arguments -->
      <div
        class="grid-w-fit mt-1 grid grid-cols-[minmax(40px,auto)_1fr] gap-x-4 border border-orange-900 border-opacity-[12%] p-3"
      >
        <template v-for="field in inputFields" :key="field.id">
          <div class="flex flex-row gap-1 py-1">
            <span>{{ field.name }}</span>
            <span class="text-gray-400">{{ renderSimpleType(field) }}</span>
          </div>
          <InlineValueCell
            :ref="(el: any) => argumentsGrid.registerColumnRef(field?.id, 'value', el)"
            :model-value="arguments_[field.name as string]"
            @update:model-value="(val: any) => setArgument(field.name as string, val)"
            :type="field"
            :readonly="false"
            :placeholder-value="field.name"
            immediate
            @navigate-left="argumentsGrid.navigateLeft(field?.id, 'value')"
            @navigate-right="argumentsGrid.navigateRight(field?.id, 'value')"
            @navigate-up="argumentsGrid.navigateUp(field?.id, 'value')"
            @navigate-down="argumentsGrid.navigateDown(field?.id, 'value')"
            class="my-0.5 w-full self-start rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-100"
          />
          <!-- :EditableCellStyle -->
        </template>
      </div>
    </div>
    <!-- Down arrow in the middle -->
    <div class="my-2 flex w-full flex-row justify-center">
      <button
        ref="runButtonRef"
        class="p-1 text-orange-600 outline-none hover:bg-orange-100 focus:bg-orange-100"
        @click="run"
        @keydown.enter.prevent="run"
        @keydown.space.prevent="run"
        @keydown.up.prevent="argumentsGrid.focus(-1, 'value')"
      >
        <ArrowDownIcon class="h-6 w-6" />
      </button>
    </div>
    <!-- Current/last output  -->
    <div class="relative mx-auto min-h-[100px] w-full max-w-[800px] border border-orange-900 border-opacity-[12%]">
      <div class="p-2" v-if="lastOutput != null">
        <InlineValueCell
          v-if="outputField"
          :type="outputField"
          :model-value="lastOutput"
          :readonly="true"
          :immediate="false"
        />
      </div>
      <div v-else class="flex h-full w-full flex-col items-center justify-center">
        <div class="p-2 text-gray-500">
          No output yet. You should
          <button
            class="text-gray-700 underline decoration-dashed underline-offset-2 hover:bg-orange-100 hover:text-gray-900 hover:decoration-solid focus:bg-orange-100"
            @click="run"
          >
            run</button
          >.
        </div>
      </div>
    </div>
    <!-- Runs -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="mt-6 flex flex-row items-baseline gap-1">
        <button
          class="text-2xl font-bold text-gray-900 decoration-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
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
