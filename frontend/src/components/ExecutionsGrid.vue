mapNameVersion
<template>
  <div class="mt-8 flex flex-col">
    <div class="-my-2 -mx-4 sm:-mx-6 lg:-mx-8">
      <div class="inline-block min-w-full py-2 align-middle">
        <div class="shadow-sm ring-1 ring-black ring-opacity-5">
          <table class="w-full border-separate" style="border-spacing: 0">
            <thead class="bg-gray-50">
              <tr>
                <th
                  scope="col"
                  v-for="column in columns"
                  :key="column.name"
                  class="sticky top-0 z-10 border-b border-gray-300 bg-gray-50 bg-opacity-75 py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 backdrop-blur backdrop-filter sm:pl-6 lg:pl-8"
                >
                  {{ column.name }}
                </th>
              </tr>
            </thead>
            <tbody class="bg-white">
              <tr v-for="(row, rowIdx) in rows" :key="row.id">
                <td
                  :class="[
                    rowIdx !== rows.length - 1 ? '' : '',
                    'w-4 whitespace-nowrap border-b border-gray-200 py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6 lg:pl-8',
                  ]"
                >
                  <div class="inline-flex flex-col">
                    {{ row["state"] }}
                    <span class="font-normal text-gray-700">{{ row["updated"] }}</span>
                    <span class="font-normal text-gray-700">{{ row["duration"] || "..." }}</span>
                  </div>
                </td>
                <td
                  v-for="column in datasetColumns"
                  :key="column.key"
                  :class="[
                    rowIdx !== rows.length - 1 ? 'border-b border-gray-200' : '',
                    'whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6 lg:pl-8',
                  ]"
                >
                  <template v-if="row['datasets'][column.key]?.result.value != null">
                    <RecordsPreview
                      :style="'preview'"
                      :fields="specFor(column.artifact)"
                      :records="row['datasets'][column.key]?.result.value['results']"
                    />
                  </template>
                  <template v-else-if="row['datasets'][column.key]?.error">
                    {{ row["datasets"][column.key]?.error }}
                  </template>
                  <template v-else-if="row['datasets'][column.key]?.loading"> ... </template>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import { useNow } from "@/composables/useNow";
import { computedAsync, useArtifactsStore, type AsyncResult } from "@/stores";
import {
  getAllConnectedDatasets,
  isFieldSpec,
  isFieldType,
  makeFieldSpec,
  type ArtifactVersion,
  type DatasetMetadata,
  type Execution,
  type ExecutionArtifactConnection,
  type FieldSpec,
  type FieldType,
  type LimitPaginatedResult,
} from "@/types";
import { mapNameVersion, toNameVersion } from "@/utils/versioning";
import { DateTime, type ToRelativeOptions } from "luxon";
import qs from "qs";
import { computed } from "vue";
import RecordsPreview from "./RecordsPreview.vue";

const props = defineProps<{
  executions: Array<Execution>;
}>();

// trim execution artifact name to shortest unambiguous identifier for legibility
function datasetToFriendlyName(execution: Execution, connection: ExecutionArtifactConnection) {
  let [dataset] = mapNameVersion(connection.artifact);
  if (execution.type == "flow") {
    const [flowName, _] = mapNameVersion(execution.flow);
    if (dataset.startsWith(flowName)) {
      dataset = dataset.split(".", 2)[1];
    }
  }
  return dataset;
}

const datasetColumns = computed(() => {
  // use "schema" of latest completed execution (for now)
  const schemaExecution = props.executions.find((execution) => execution.state == "completed");
  return schemaExecution == null
    ? []
    : getAllConnectedDatasets(schemaExecution).map((connection) => {
        const [dataset] = mapNameVersion(connection.artifact);
        return {
          name: datasetToFriendlyName(schemaExecution, connection),
          key: dataset,
          connection: connection,
          artifact: connection.artifact,
        };
      });
});

type PaginatedDataset = LimitPaginatedResult<Record<string, any>>;

const luxonToRelativeOptions: ToRelativeOptions = {
  locale: "en-US",
  style: "narrow",
  unit: ["years", "months", "weeks", "days", "hours", "minutes"],
};
const columns = computed(() => [{ name: "state" }, ...datasetColumns.value]);

const now = useNow();
function getTimeFromNow(dt: DateTime): string | null {
  if (now.value.diff(dt, "seconds").seconds < 60) {
    return "just now";
  } else {
    return dt.toRelative({ ...luxonToRelativeOptions, base: now.value });
  }
}

const rows = computed(() =>
  props.executions.map((execution) => {
    const datasets: Record<string, AsyncResult<PaginatedDataset>> = getAllConnectedDatasets(
      execution
    )
      .map((connection) => {
        const [dataset, version] = mapNameVersion(connection.artifact);
        // cache preview dataset, may be enough
        if (connection.dataset_preview) {
          cacheDataset(
            dataset,
            version,
            connection.view?.data || connection.view_inline,
            connection.dataset_preview.limit,
            connection.dataset_preview
          );
        }
        const result = computedAsync(() =>
          readDataset(dataset, version, connection.view?.data || connection.view_inline)
        );
        return { dataset, result };
      })
      .reduce((previous, { dataset, result }) => ({ ...previous, [dataset]: result }), {});
    return {
      id: execution.id,
      state: execution.state,
      started: execution.started_at ? getTimeFromNow(DateTime.fromISO(execution.started_at)) : null,
      updated: execution.updated_at ? getTimeFromNow(DateTime.fromISO(execution.updated_at)) : null,
      duration:
        execution.terminated_at && execution.started_at
          ? DateTime.fromISO(execution.terminated_at)
              .diff(DateTime.fromISO(execution.started_at))
              .toMillis() + "ms"
          : null,
      datasets: datasets,
    };
  })
);

function unravelSpec(spec: FieldSpec): FieldSpec[] {
  if (isFieldType(spec.type)) {
    return [makeFieldSpec("", spec.type as FieldType)];
  } else if (isFieldSpec(spec.type)) {
    // if the spec type is just a blank spec it's just a level of annotation
    return unravelSpec(spec.type as FieldSpec);
  } else if (Array.isArray(spec.type)) {
    return spec.type;
  } else {
    return Object.values(spec.type);
  }
}

function specFor(dataset: string): FieldSpec[] {
  if (datasetVersions.value == null || datasetVersions.value[dataset] == null) {
    // TODO @Cleanup: derive spec from values?
    return [];
  }

  const datasetVersion = datasetVersions.value[dataset];
  const spec = (datasetVersion.metadata as DatasetMetadata).record_spec;
  if (spec == null || spec.type == {}) {
    // TODO @Cleanup: fall back to above
    return [];
  }

  return unravelSpec(spec);
}

const artifactsStore = useArtifactsStore();

const cachedDatasetVersions: Record<string, ArtifactVersion> = {};
const { result: datasetVersions } = computedAsync(async () => {
  const datasets = await Promise.all(
    datasetColumns.value
      .map((column) => column.artifact)
      .map(async (artifact) => {
        if (cachedDatasetVersions[artifact] != null) {
          return cachedDatasetVersions[artifact];
        }
        const datasetVersion = await artifactsStore.getVersion(...mapNameVersion(artifact));
        cachedDatasetVersions[artifact] = datasetVersion;
        return datasetVersion;
      })
  );
  const datasetVersions: Record<string, ArtifactVersion> = {};
  datasets.forEach((dataset) => (datasetVersions[toNameVersion(dataset)] = dataset));
  return datasetVersions;
});

// TODO @Robustness @Performance: consolidate and improve dataset/record fetching
const cachedDatasetsRecords: Record<string, PaginatedDataset> = {};

function getRecordsId(
  dataset: string,
  version: string,
  view_data: Record<string, any> | undefined,
  limit: number
): string {
  if (view_data != null) {
    return `${dataset}@${version}<${qs.stringify(view_data)}>[:${limit}]`;
  } else {
    return `${dataset}@${version}[:${limit}]`;
  }
}

function cacheDataset(
  dataset: string,
  version: string,
  view_data: Record<string, any> | undefined,
  limit: number,
  result: PaginatedDataset
) {
  const recordsId = getRecordsId(dataset, version, view_data, limit);
  cachedDatasetsRecords[recordsId] = result;
}

async function readDataset(
  dataset: string,
  version: string,
  view_data?: Record<string, any>,
  limit = 3
): Promise<PaginatedDataset> {
  const recordsId = getRecordsId(dataset, version, view_data, limit);
  const cachedDataset = cachedDatasetsRecords[recordsId];
  if (cachedDataset != null) {
    return Promise.resolve(cachedDataset);
  }

  dataset = encodeURIComponent(dataset);
  version = encodeURIComponent(version);
  return api
    .get<PaginatedDataset>(`/datasets/${dataset}/versions/${version}/records`, {
      params: { limit, ...view_data },
    })
    .then((response) => response.data)
    .then((dataset) => {
      cachedDatasetsRecords[recordsId] = dataset;
      return dataset;
    });
}
</script>
