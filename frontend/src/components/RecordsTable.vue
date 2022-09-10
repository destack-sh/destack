<template>
  <div class="mt-8 flex flex-col">
    <div class="-my-2 -mx-4 sm:-mx-6 lg:-mx-8">
      <div class="inline-block min-w-full py-2 align-middle">
        <div class="shadow-sm ring-1 ring-black ring-opacity-5">
          <table class="min-w-full border-separate" style="border-spacing: 0">
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
              <tr v-for="(row, recordIdx) in records" :key="recordIdx">
                <td
                  v-for="column in columns"
                  :key="column.name"
                  :class="[
                    recordIdx !== records.length - 1 ? 'border-b border-gray-200' : '',
                    'whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6 lg:pl-8',
                  ]"
                >
                  <SpanTextDisplay v-if="(row as any).text != undefined" :model-value="row" :spec="props.fields" />
                  <template v-else>{{ row.data[column.name] }}</template>
                </td>
              </tr>
              <tr class="bg-white" v-if="records.length == 0">
                <td :colspan="columns.length">
                  <slot name="empty" />
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
import type { DatasetRecord, FieldSpec } from "@/types";
import { unravelSpec } from "@/utils/spec";
import { computed } from "vue";
import SpanTextDisplay from "../displays/SpanTextDisplay.vue";
const props = defineProps<{
  spec: FieldSpec;
  records: Array<DatasetRecord>;
}>();

const columns = computed(() => unravelSpec(props.spec));
</script>
