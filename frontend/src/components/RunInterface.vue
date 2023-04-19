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
import { buildsOf, symbolOf, symbolsLike } from "@/state/runtime";
import { QuestionMarkCircleIcon, XMarkIcon } from "@heroicons/vue/20/solid";
import { PlayIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { computed, inject, nextTick, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{ runnableId: string; runnableType: SymbolType }>();

const symbol = computed(() => symbolOf(props.runnableId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((n) => !n.isOutput) ?? []);
const outputFields = computed(() => symbol.value?.typeNodes?.filter((n) => n.isOutput) ?? []);
const includeAncestorVersions = ref(true);

// local run interface state
const state = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
if (state == null) {
  throw new Error("need interface state context");
}
// input source
// batch mode (with computed setter on state)
const batchMode = computed({
  get() {
    return state.get("batchMode", false);
  },
  set(value) {
    state?.set("batchMode", value);
  },
});
// batch source dataset (also with setter, lookup as symbol in getter)
// TODO @UX: filter available datasets to correct types
const availableDatasets = symbolsLike({ symbolTypes: [SymbolType.Data], types: [StatementType.Definition] });
const batchSourceDataset = computed({
  get() {
    return symbolOf(state.get("batchSourceDatasetId", ""));
  },
  set(value) {
    state?.set("batchSourceDatasetId", value?.id);
  },
});

// arguments
const arguments_: Ref<Record<string, any>> = computed(() => state.get("arguments", {}) as Record<string, any>);
function setArgument(key: string, value: string) {
  const args = { ...arguments_.value };
  args[key] = value;
  state?.set("arguments", args);
}

// builds
const builds: Ref<InterpSymbol[]> = computed(() =>
  state
    .get("buildIds", [])
    .map((id) => symbolOf(id))
    .filter((b) => b != null)
);
const availableBuilds = buildsOf(symbol);
const remainingAvailableBuilds = computed(() =>
  availableBuilds.value.filter((b) => !builds.value.some((b2) => b2.id == b.id))
);
function startAddBuild() {
  addingBuild.value = true;
  nextTick(() => addBuildRef.value?.focus);
}
function addBuild(build: InterpSymbol) {
  // add build if it doesn't exist yet
  if (builds.value.find((b) => b.id == build.id) == null) {
    state?.set("buildIds", [...state.get("buildIds", []), build.id]);
  }
  addingBuild.value = true;
}
function removeBuild(build: InterpSymbol) {
  state?.set(
    "buildIds",
    state.get("buildIds", []).filter((id) => id != build.id)
  );
}
const addingBuild = ref(false);

// auto-set build if available and not set
watchEffect(() => {
  if (builds.value.length == 0 && availableBuilds.value.length > 0) {
    addBuild(availableBuilds.value[0]);
  }
});

const runsTableRef = ref<InstanceType<typeof RunsTable> | null>(null);
const runButtonRef = ref<HTMLButtonElement | null>(null);
const selectBuildRef = ref<InstanceType<typeof ReferenceComboCell> | null>(null);
const addBuildRef = ref<InstanceType<typeof ReferenceComboCell> | null>(null);
const argumentsGrid = useNavigationGrid<"value", InstanceType<typeof InlineValueCell>>(
  computed(() => ["value"]),
  inputFields,
  {
    gridNavigateUp: () => selectBuildRef.value?.focus(),
    gridNavigateDown: () => runButtonRef.value?.focus(),
  }
);
const lastOutputByBuild: Ref<Record<string, any>> = ref({});
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
  const runOptions = { block: true, timeoutSeconds: 120 };

  // single executions
  if (!batchMode.value) {
    // TODO @Performance: parallelize execution across builds
    for (const build of builds.value) {
      const ret = await ops.runtime.run(symbol.value.id, builds.value?.[0].id, args, runOptions);
      if (ret?.errors || ret?.data?.run.__typename != "RunState" || !ret?.data?.run.success) {
        notifications.show({
          type: "run.fail",
          kind: "error",
          message: "Run failed",
          description: `Failed to run ${symbol.value?.name}: ${ret?.data?.run?.error ?? "rejected"}`,
        });
        lastOutputByBuild.value[build.id] = null;
      } else {
        lastOutputByBuild.value[build.id] = ret.data.run.output;
      }
    }
  } else {
    // TODO @Incomplete: batch mode
  }

  lastOutputDirty.value = false;
}

function openRunsEditor() {
  const e = editor.openRuns({ create: true });
  editor.focusEditor(e);
}

const inputColumns = computed(() =>
  inputFields.value == null ? undefined : inputFields.value.map((f) => ({ name: f.name, type: f }))
);
const outputColumns = computed(() =>
  outputFields.value == null ? undefined : outputFields.value.map((f) => ({ name: f.name, type: f }))
);
</script>
<template>
  <div
    class="flex flex-col items-baseline bg-white px-12 py-6"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <!-- Runconfig -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="flex flex-row items-baseline gap-1">
        <!-- the runnable should maybe be configurable, but that would require mutating the editor instance -->
        <span class="text-3xl font-bold text-gray-900">Run {{ symbol?.name }}</span>
      </h2>
      <h3 class="mt-8 text-2xl font-bold text-gray-900">Quick run</h3>
      <!-- Input source & builds -->
      <div class="mt-2 flex flex-row justify-between">
        <div class="flex flex-row items-baseline">
          <!-- Input source (single vs batch) -->
          <button
            class="rounded-sm rounded-r-none border border-orange-900 border-opacity-[15%] px-2 py-1 hover:cursor-pointer"
            :class="{ 'bg-orange-100': !batchMode }"
            @click="batchMode = false"
          >
            Single
          </button>
          <button
            class="rounded-sm rounded-l-none border border-orange-900 border-opacity-[15%] px-2 py-1 hover:cursor-pointer"
            :class="{ 'rounded-r-none border-r-0 bg-orange-100': batchMode }"
            @click="batchMode = true"
          >
            Batch
          </button>
          <!-- Batch source select (if batch mode) -->
          <ReferenceComboCell
            v-if="batchMode"
            class="rounded-sm rounded-l-none border border-orange-900 border-opacity-[15%] bg-orange-100 py-1 pr-2 hover:cursor-pointer"
            :class="{ 'rounded-l-none border-l-0': batchMode }"
            ref="selectDatasetRef"
            :reference="batchSourceDataset"
            @set-reference="batchSourceDataset = $event ?? undefined"
            :available-symbols="availableDatasets"
            @navigate-down="argumentsGrid.focus(0, 'value')"
          />
          <!-- Builds -->
          <span
            class="ml-3 flex items-center gap-0.5 border border-orange-900 border-opacity-[15%] bg-orange-100 px-2 py-1"
            v-for="build in builds"
            :key="build.id"
          >
            {{ build.name }}
            <button v-if="builds.length > 1" class="text-gray-400 hover:text-gray-800" @click="removeBuild(build)">
              <XMarkIcon class="h-4 w-4" />
            </button>
          </span>
          <button
            v-if="remainingAvailableBuilds.length > 0 && !addingBuild"
            class="ml-2 p-1 text-gray-900 hover:bg-orange-100"
            @click="startAddBuild"
          >
            <PlusIcon class="-mb-0.5 h-4 w-4" />
          </button>
          <ReferenceComboCell
            v-if="addingBuild"
            ref="addBuildRef"
            class="ml-2"
            @set-reference="$event != null && addBuild($event)"
            :available-symbols="remainingAvailableBuilds"
            @navigate-down="argumentsGrid.focus(0, 'value')"
          />
        </div>

        <!-- Run / deploy hint -->
        <div class="flex flex-row gap-3">
          <!-- Deploy hint -->
          <span class="flex flex-row items-center gap-0.5">
            <button
              class="rounded-sm px-1 text-gray-700 hover:bg-orange-100 hover:text-gray-900"
              @click="actions.apply('version.deploy')"
            >
              Deploy
            </button>
            <router-link
              to="/symbolx/docs#Deploying"
              class="text-gray-300 hover:bg-orange-100 hover:text-gray-700"
              target="_blank"
            >
              <QuestionMarkCircleIcon class="h-4 w-4" />
            </router-link>
          </span>
          <!-- Run button -->
          <button
            ref="runButtonRef"
            class="flex flex-row items-center justify-center gap-1 p-1 text-orange-600 outline-none hover:bg-orange-100 focus:bg-orange-100"
            @click="run"
            @keydown.enter.prevent="run"
            @keydown.space.prevent="run"
            @keydown.up.prevent="argumentsGrid.focus(-1, 'value')"
          >
            Run
            <PlayIcon class="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
    <!-- Single -->
    <div v-if="!batchMode" class="mx-auto mt-2 w-full max-w-[800px]">
      <div
        class="grid-w-fit mt-1 grid w-full grid-cols-[minmax(40px,auto)_1fr] gap-x-4 border border-orange-900 border-opacity-[12%] p-3"
      >
        <template v-for="field in inputFields" :key="field.id">
          <div class="flex flex-row gap-1 py-1">
            <span class="font-bold">{{ field.name }}</span>
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
            class="my-0.5 w-full self-start border border-transparent py-0.5 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100"
          />
          <!-- :EditableCellStyle -->
        </template>
      </div>
      <!-- Current/last output  -->
      <!-- TODO @UX: rework multi-build output -->
      <div class="mt-4" v-for="build in builds" :key="build.id">
        <span class="border border-orange-900 border-opacity-[12%] bg-orange-100 px-3 py-1">{{ build.name }}</span>
        <div
          class="grid-w-fit relative mt-2 grid w-full grid-cols-[minmax(40px,auto)_1fr] gap-x-4 border border-orange-900 border-opacity-[12%] p-3"
        >
          <template v-if="lastOutputByBuild[build.id] != null">
            <template v-for="field in outputFields" :key="field.id">
              <div class="flex flex-row gap-1 py-1">
                <span class="font-bold">{{ field.name }}</span>
              </div>
              <InlineValueCell
                :type="field"
                :model-value="lastOutputByBuild[build.id][field.name as string]"
                :readonly="true"
                :immediate="false"
              />
            </template>
          </template>
          <div v-else class="flex h-full w-full flex-col items-center justify-center">
            <div class="text-gray-500">No output yet</div>
          </div>
        </div>
      </div>
    </div>
    <!-- Batch -->
    <div v-else class="mx-auto mt-2 w-full max-w-[800px]">TODO</div>
    <!-- TODO -->
    <!-- Past runs -->
    <div class="mx-auto mt-8 w-full max-w-[800px]">
      <h2 class="flex flex-row items-baseline gap-1">
        <button
          class="text-2xl font-bold text-gray-900 decoration-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="openRunsEditor"
        >
          Runs
        </button>
        <span class="rounded-3xl bg-gray-100 px-1 py-0.5 text-sm text-gray-900" v-if="runsTableRef">
          {{ humanizeNumber(runsTableRef?.totalCount) }}
        </span>
      </h2>
      <RunsTable
        class="mt-2"
        ref="runsTableRef"
        :project-id="editor.currentProjectId"
        :project-version-id="editor.currentProjectVersionId"
        :include-ancestor-versions="includeAncestorVersions"
        :build-ids="state?.get('buildIds', [])"
        :task-ids="runnableType == SymbolType.Task ? [runnableId] : undefined"
        :code-ids="runnableType == SymbolType.Code ? [runnableId] : undefined"
        :input-columns="inputColumns"
        :output-columns="outputColumns"
        override-from-props
      />
    </div>
  </div>
</template>
