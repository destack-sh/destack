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
                  {{ row["state"] }}
                  <br />
                  <span class="font-normal text-gray-700">{{ row["updated"] }}</span>
                </td>
                <td
                  v-for="column in artifactColumns"
                  :key="column.key"
                  :class="[
                    rowIdx !== rows.length - 1 ? 'border-b border-gray-200' : '',
                    'whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6 lg:pl-8',
                  ]"
                >
                  <template v-if="row['artifacts'][column.key]?.result != null">
                    {{ row["artifacts"][column.key]?.result["results"] }}
                  </template>
                  <template v-else-if="row['artifacts'][column.key]?.error">
                    {{ row["artifacts"][column.key]?.error }}
                  </template>
                  <template v-else-if="row['artifacts'][column.key]?.loading"> ... </template>
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
import {
  mapArtifactNameVersion,
  type Execution,
  type LimitPaginatedResult as PaginatedResult,
} from "@/types";
import { DateTime, type ToRelativeOptions } from "luxon";
import { computed } from "vue";

const props = defineProps<{
  executions: Array<Execution>;
}>();

const artifactColumns = computed(() =>
  props.executions.length == 0
    ? []
    : props.executions[0].connected_artifacts.map((connection) => {
        const [dataset] = mapArtifactNameVersion(connection.artifact);
        return {
          name: dataset,
          key: dataset,
          connection: connection,
          artifact: connection.artifact,
        };
      })
);

type PaginatedDataset = PaginatedResult<Record<string, any>>;

const luxonToRelativeOptions: ToRelativeOptions = { locale: "en-US", style: "short" };
const columns = computed(() => [{ name: "state" }, ...artifactColumns.value]);
const rows = computed(() =>
  props.executions.map((execution) => {
    const artifacts: Record<string, AsyncResult<PaginatedDataset>> = execution.connected_artifacts
      .map((connection) => {
        const [dataset, version] = mapArtifactNameVersion(connection.artifact);
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
          ? DateTime.fromISO(execution.terminated_at).toRelative({
              base: DateTime.fromISO(execution.started_at),
              ...luxonToRelativeOptions,
            })
          : null,
      artifacts,
    };
  })
);

async function readDataset(dataset: string, version: string, limit = 3): Promise<PaginatedDataset> {
  dataset = encodeURIComponent(dataset);
  version = encodeURIComponent(version);
  return api
    .get<PaginatedDataset>(`/datasets/${dataset}/versions/${version}/records`, {
      params: { limit },
    })
    .then((response) => response.data);
}
</script>
