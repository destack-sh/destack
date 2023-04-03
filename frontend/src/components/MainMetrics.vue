<script lang="ts" setup>
import { EvaluationScope, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useEvaluations } from "@/state/evaluations";
import { buildsOf, useCurrentInterpModule } from "@/state/runtime";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
}>();

const editor = useEditorState();
const runtime = useCurrentInterpModule();
const mainSymbol = computed(() => runtime.moduleIndex.value?.symbolsById[editor.mainSymbolId ?? ""]);
const mainBuilds = buildsOf(mainSymbol as Ref<{ id: string; parentId: string } | undefined>);

type Metric = {
  label: string;
  description: string;
  value: number | string;
  unit?: string;
  stale: boolean;
};

type MetricSet = {
  label: string;
  description: string;
  metrics: Metric[];
};

function toPercent(value?: number, alt = "??"): string {
  return value != null ? (value * 100).toFixed(0) : alt;
}

function toFixed(value?: number, alt = "??"): string {
  return value != null ? value.toFixed(1) : alt;
}

// global to scope
const globalEvaluations = useEvaluations(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    scopeIn: ref([EvaluationScope.Module]),
  },
  { live: true }
);
const globalEvaluation = computed(() =>
  (globalEvaluations.evaluations.value?.length ?? 0) > 0 ? globalEvaluations.evaluations.value[0] : undefined
);
const globalMetricSet: Ref<MetricSet | null> = computed(() => {
  if (globalEvaluation.value == null || globalEvaluation.value.aggregatedMetrics?.clarity == null) {
    return null;
  }
  const metrics = globalEvaluation.value.aggregatedMetrics;
  return {
    label: "Bench",
    description: `Bench-wide analysis of '${mainSymbol.value?.name ?? runtime.name.value}'.`,
    metrics: [
      {
        label: "Clarity",
        description: "How comprehensible the instructions is.",
        value: toPercent(metrics["clarity"]),
        unit: "%",
        stale: false, // TODO @UX: track global interp/lint staleness
      },
      {
        label: "Difficulty",
        description: "How complex the instruction is.",
        value: toFixed(metrics["difficulty"]),
        unit: "x",
        stale: false,
      },
    ],
  };
});

// local to a build for scope
const buildEvaluations = useEvaluations(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    scopeIn: computed(() =>
      mainSymbol.value == null || mainSymbol.value?.symbolType == SymbolType.Build
        ? [EvaluationScope.Build]
        : [EvaluationScope.Instruction]
    ),
    buildIdIn: computed(() => mainBuilds.value.map((b) => b.id)),
    systemIdIn: computed(() => (mainSymbol.value?.symbolType == SymbolType.Task ? [mainSymbol.value?.id] : null)),
  },
  { live: true }
);

function getBuildEvaluation(buildId: string, symbolId?: string) {
  return buildEvaluations.evaluations.value?.find(
    (e) => e.build?.id == buildId && (symbolId == null || e.statement?.id == symbolId)
  );
}

const buildMetricSets: Ref<MetricSet[]> = computed(() => {
  const buildMetricSets: MetricSet[] = [];
  for (const build of mainBuilds.value) {
    const nonBuildSymbolId = mainSymbol.value?.symbolType != SymbolType.Build ? mainSymbol.value?.id : null;
    const buildEvaluation = getBuildEvaluation(build.id, nonBuildSymbolId);
    if (buildEvaluation == null) {
      continue;
    }
    const metrics = buildEvaluation.aggregatedMetrics;
    const stale = runtime.staleSymbols.value?.find((s) => s.id == build.id) != null;
    const buildMetrics: Metric[] = [
      {
        label: "Performance",
        description: "How well the AI does.",
        value: toPercent(metrics["performance"]),
        unit: "%",
        stale,
      },
    ];
    if (nonBuildSymbolId != null) {
      buildMetrics.push({
        label: "Speed",
        description: "How fast the AI is.",
        value: toFixed(metrics["speed"]),
        unit: "/min",
        stale,
      });
    }

    buildMetricSets.push({
      label: build.name ?? "???",
      description: `'${build.name}' on '${mainSymbol.value?.name}'.`,
      metrics: buildMetrics,
    });
  }

  return buildMetricSets;
});

const metricSets = computed(() => {
  const metricSets = [];
  if (globalMetricSet.value != null) {
    metricSets.push(globalMetricSet.value);
  }
  metricSets.push(...buildMetricSets.value);
  return metricSets;
});
</script>
<template>
  <div class="flex flex-row gap-3">
    <div v-for="metricSet in metricSets" :key="metricSet.label" class="group relative rounded-sm">
      <!-- Metric set itself -->
      <button
        class="relative flex flex-row gap-1 rounded-sm border border-transparent px-2 hover:cursor-pointer hover:border-sky-900 hover:border-opacity-[12%] hover:bg-sky-100"
      >
        <!-- Metric set label for builds (if more than one) -->
        <span
          v-if="metricSets.length > 2 && metricSet.label != 'Bench'"
          class="absolute left-0 -top-1.5 z-[5] mx-auto w-full text-center text-xs text-sky-900"
        >
          <!-- TODO @UX: clean up multi-build metrics -->
          <span class="rounded-sm border-sky-900 border-opacity-[12%] p-0.5 py-0 text-xs">{{ metricSet.label }} </span>
        </span>
        <!-- Metric set -->
        <div
          v-for="metric in metricSet.metrics"
          :key="metric.label"
          class="relative flex flex-row items-start gap-1 p-1.5 text-center"
        >
          <!-- Metric -->
          <span class="text-sm font-bold text-gray-900">{{ metric.value }} </span>
          <!-- Label -->
          <span class="text-xs font-bold text-gray-500">{{ metric.label.slice(0, 1) }}</span>
          <!-- Staleness indicator -->
          <span v-if="metric.stale" class="absolute right-1.5 top-4 font-bold text-yellow-600">*</span>
        </div>
      </button>
      <!-- Metric set hover popover -->
      <div
        class="invisible absolute top-10 z-20 w-80 rounded-sm bg-white px-3 py-2 shadow-sm ring-1 ring-sky-900 ring-opacity-40 group-hover:visible"
      >
        <h3 class="text-sm font-bold">{{ metricSet.label }} metrics</h3>
        <p class="text-sm text-gray-500">{{ metricSet.description }}</p>
        <!-- Metric breakdown -->
        <ul class="mt-4 flex flex-col gap-3">
          <li v-for="metric in metricSet.metrics" :key="metric.label" class="flex flex-row justify-between">
            <!-- Label & description -->
            <span class="flex flex-col gap-0.5">
              <span class="text-sm font-bold text-gray-900">{{ metric.label }}</span>
              <span class="text-sm text-gray-500">
                {{ metric.description }}
              </span>
            </span>
            <!-- Value -->
            <span class="flex flex-col text-right text-sm">
              <span class="font-bold text-gray-900">{{ metric.value }}</span>
              <span class="text-sm text-gray-500" v-if="metric.unit">{{ metric.unit }}</span>
            </span>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>
