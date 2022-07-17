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
                    <span class="font-normal text-gray-700">{{ row["duration"] }}</span>
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
                      :fields="specFor(column.key)"
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
import { computedAsync, type AsyncResult } from "@/stores";
import type {
  Execution,
  ExecutionArtifactConnection,
  FieldSpec,
  LimitPaginatedResult,
  ValueType,
} from "@/types";
import { mapNameVersion } from "@/utils/versioning";
import { DateTime, type ToRelativeOptions } from "luxon";
import { computed } from "vue";
import RecordsPreview from "./RecordsPreview.vue";

const props = defineProps<{
  executions: Array<Execution>;
}>();

function getAllConnectedArtifacts(execution: Execution): ExecutionArtifactConnection[] {
  const connectedArtifacts = [...execution.connected_artifacts];
  if (execution.children != null) {
    for (const childExecution of execution.children) {
      connectedArtifacts.push(...childExecution.connected_artifacts);
    }
  }
  return connectedArtifacts;
}

function getAllConnectedDatasets(execution: Execution): ExecutionArtifactConnection[] {
  return getAllConnectedArtifacts(execution).filter(
    (connection) => connection.dataset_preview != null
  );
}

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

const luxonToRelativeOptions: ToRelativeOptions = { locale: "en-US", style: "short" };
const columns = computed(() => [{ name: "state" }, ...datasetColumns.value]);
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
            connection.dataset_preview.limit,
            connection.dataset_preview
          );
        }
        const result = computedAsync(() => readDataset(dataset, version));
        return { dataset, result };
      })
      .reduce((previous, { dataset, result }) => ({ ...previous, [dataset]: result }), {});
    return {
      id: execution.id,
      state: execution.state,
      started: execution.started_at
        ? DateTime.fromISO(execution.started_at).toRelative(luxonToRelativeOptions)
        : null,
      updated: execution.updated_at
        ? DateTime.fromISO(execution.updated_at).toRelative(luxonToRelativeOptions)
        : null,
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

function specFor(dataset: string): FieldSpec[] {
  // TODO @Feature: derive input spec from given artifacts/datasets?
  return [
    {
      _type: "FieldSpec",
      name: "text",
      description: "any text",
      type: {
        _type: "ValueType",
        dtype: "string",
      } as ValueType,
    },
  ];
}

// TODO @Robustness @Performance: consolidate and improve dataset/record fetching
const cachedDatasets: Record<string, PaginatedDataset> = {};

function cacheDataset(dataset: string, version: string, limit: number, result: PaginatedDataset) {
  const recordsId = `${dataset}@${version}[:${limit}]`;
  cachedDatasets[recordsId] = result;
}

async function readDataset(dataset: string, version: string, limit = 3): Promise<PaginatedDataset> {
  const recordsId = `${dataset}@${version}[:${limit}]`;
  const cachedDataset = cachedDatasets[recordsId];
  if (cachedDataset != null) {
    return Promise.resolve(cachedDataset);
  }

  dataset = encodeURIComponent(dataset);
  version = encodeURIComponent(version);
  return api
    .get<PaginatedDataset>(`/datasets/${dataset}/versions/${version}/records`, {
      params: { limit },
    })
    .then((response) => response.data)
    .then((dataset) => {
      cachedDatasets[recordsId] = dataset;
      return dataset;
    });
}
</script>
