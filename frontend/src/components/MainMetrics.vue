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
  value: number;
  unit?: string;
};

type MetricSet = {
  label: string;
  description: string;
  metrics: Metric[];
};

// global to scope
const globalEvaluations = useEvaluations({
  projectId: toRef(props, "projectId"),
  projectVersionId: toRef(props, "projectVersionId"),
  scopeIn: ref([EvaluationScope.Module]),
});
const globalEvaluation = computed(() =>
  (globalEvaluations.evaluations.value?.length ?? 0) > 0 ? globalEvaluations.evaluations.value[0] : undefined
);
const globalMetricSet: Ref<MetricSet | null> = computed(() => {
  if (globalEvaluation.value == null) {
    return null;
  }
  const metrics = globalEvaluation.value.aggregatedMetrics;
  return {
    label: "General",
    description: `Bench-wide analysis of '${mainSymbol.value?.name ?? runtime.name.value}'.`,
    metrics: [
      {
        label: "Clarity",
        description: "How comprehensible the instructions is.",
        value: metrics["clarity"] != null ? (metrics["clarity"] * 100).toFixed(0) : "??",
        unit: "%",
      },
      {
        label: "Difficulty",
        description: "How complex the instruction is.",
        value: metrics["difficulty"] != null ? metrics["difficulty"].toFixed(1) : "??",
        unit: "x",
      },
    ],
  };
});

// local to a build for scope
const buildEvaluations = useEvaluations({
  projectId: toRef(props, "projectId"),
  projectVersionId: toRef(props, "projectVersionId"),
  scopeIn: ref([EvaluationScope.Build]),
  buildIdIn: computed(() => mainBuilds.value.map((b) => b.id)),
  systemIdIn: computed(() => (mainSymbol.value?.symbolType == SymbolType.Task ? [mainSymbol.value?.id] : null)),
});

function getBuildEvaluation(buildId: string) {
  return buildEvaluations.evaluations.value?.find((e) => e.build?.id == buildId);
}

const buildMetricSets: Ref<MetricSet[]> = computed(() => {
  const buildMetricSets: MetricSet[] = [];
  for (const build of mainBuilds.value) {
    const buildEvaluation = getBuildEvaluation(build.id);
    if (buildEvaluation == null) {
      continue;
    }
    const buildMetrics = [
      {
        label: "Performance",
        description: "How well the AI does.",
        value: 90,
        unit: "%",
      },
      {
        label: "Speed",
        description: "How fast the AI is.",
        value: 3,
        unit: "/min",
      },
    ];
    buildMetricSets.push({
      label: build.name ?? "???",
      description: `Performance of '${build.name}' on ${mainSymbol.value?.name}`,
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
        class="relative flex flex-row gap-2 rounded-sm border border-transparent px-2 hover:cursor-pointer hover:border-sky-900 hover:border-opacity-[12%] hover:bg-sky-100"
      >
        <!-- Metric set label for builds (if more than one) -->
        <span
          v-if="metricSet.label != 'general' && metricSets.length > 2"
          class="absolute left-0 -top-2 z-[5] mx-auto w-full text-center text-xs text-sky-900"
        >
          <!-- TODO @UX: clean up multi-build metrics -->
          <span class="rounded-sm border border-b-0 border-l border-sky-900 border-opacity-[12%] p-0.5 py-0 text-xs"
            >{{ metricSet.label }}
          </span>
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
        </div>
      </button>
      <!-- Popover details if hovered -->
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
