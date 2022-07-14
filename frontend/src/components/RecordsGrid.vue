<template>
  <div class="mt-8 flex flex-col">
    <div class="-my-2 -mx-4 sm:-mx-6 lg:-mx-8">
      <div class="inline-block min-w-full py-2 align-middle">
        <div class="shadow-sm ring-1 ring-black ring-opacity-5">
          <table class="min-w-full border-separate" style="border-spacing: 0">
            <thead class="bg-gray-50" v-if="displayOptions.showHeader">
              <tr>
                <th
                  scope="col"
                  v-for="column in columns"
                  :key="column.name"
                  class="sticky top-0 z-10 border-b border-gray-300 bg-gray-50 bg-opacity-75 py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 backdrop-blur backdrop-filter sm:pl-6 lg:pl-8"
                >
                  {{ column.name }}}
                </th>
              </tr>
            </thead>
            <tbody class="bg-white">
              <tr v-for="(record, recordIdx) in records" :key="record.email">
                <td
                  v-for="column in columns"
                  :key="column.name"
                  :class="[
                    recordIdx !== records.length - 1 ? 'border-b border-gray-200' : '',
                    'whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6 lg:pl-8',
                  ]"
                >
                  {{ record[column.name] }}
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
import type { FieldSpec } from "@/types";
import { computed } from "vue";
const props = defineProps<{
  fields: Array<FieldSpec>;
  records: Array<Record<string, any>>;
  style: "preview" | "full";
}>();

const columns = computed(() => props.fields);

const displayOptionsByStyle = {
  preview: {
    showHeader: false,
  },
  full: {
    showHeader: true,
  },
};
const displayOptions = computed(() => displayOptionsByStyle[props.style]);
</script>
