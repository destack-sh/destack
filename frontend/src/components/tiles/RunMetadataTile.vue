<script lang="ts" setup>
import StructTile from "@/components/tiles/StructTile.vue";
import { formatDurationSeconds } from "@/composables/useNow";
import { useFragment } from "@/gql";
import type { Run } from "@/gql/graphql";
import { FieldType } from "@/state/fragments";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { getStatusColor } from "@/state/session";
import { DateTime } from "luxon";
import { computed } from "vue";

const props = defineProps<{
  run: Run;
}>();
const module = useCurrentModule();
const fields = computed(
  () => module.statementOf(props.run.runnable?.id)?.fields.map((f) => useFragment(FieldType, f)) ?? []
);
</script>
<template>
  <div class="flex flex-col gap-1">
    <!-- About -->
    <div>
      <!-- Table with path, status, started at, terminated at -->
      <table class="w-full table-auto">
        <tbody>
          <tr>
            <td class="font-semibold">Path</td>
            <td class="text-gray-600">{{ module.pathOf(props.run.runnable as any) }}</td>
          </tr>
          <tr>
            <td class="font-semibold">Status</td>
            <td class="" :class="[getStatusColor(props.run.status)]">{{ props.run.status.toLowerCase() }}</td>
          </tr>
          <tr>
            <td class="font-semibold">Started</td>
            <td class="text-gray-600">
              {{
                props.run.startedAt == null
                  ? "-"
                  : DateTime.fromISO(props.run.startedAt).toFormat("yyyy/MM/dd HH:mm:ss.SSS")
              }}
            </td>
          </tr>
          <tr>
            <td class="font-semibold">Terminated</td>
            <td class="text-gray-600">
              {{
                props.run.terminatedAt == null
                  ? "-"
                  : DateTime.fromISO(props.run.terminatedAt).toFormat("yyyy/MM/dd HH:mm:ss.SSS")
              }}
            </td>
          </tr>
          <tr>
            <td class="font-semibold">Duration</td>
            <td class="text-gray-600">
              {{ props.run.duration == null ? "-" : formatDurationSeconds(props.run.duration) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <!-- Inputs -->
    <div class="mt-1">
      <h3 class="mb-0.5 text-sm font-semibold">Inputs</h3>
      <StructTile
        readonly
        class="w-full"
        :fields="fields.filter((f) => !(f.flags & TypeFlag.IsOutput))"
        :model-value="run.inputs ?? {}"
      />
      <span v-if="fields.filter((f) => !(f.flags & TypeFlag.IsOutput)).length == 0" class="text-gray-400"
        >No inputs</span
      >
    </div>
    <!-- Outputs -->
    <div class="mt-1">
      <h3 class="mb-0.5 text-sm font-semibold">Outputs</h3>
      <StructTile
        readonly
        class="w-full"
        :fields="fields.filter((f) => f.flags & TypeFlag.IsOutput)"
        :model-value="run.outputs ?? {}"
      />
      <span v-if="fields.filter((f) => f.flags & TypeFlag.IsOutput).length == 0" class="text-gray-400">No outputs</span>
    </div>
  </div>
</template>
