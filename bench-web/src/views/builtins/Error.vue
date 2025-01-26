<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { ErrorKind, ErrorType, type RunData, type ErrorData } from "@/proto/wire";
import { Casing, toCasing } from "@/utils/string";
import Text from "@/views/content/Text.vue";

const props = defineProps<{ run?: RunData; error: ErrorData }>();
</script>
<template>
  <div class="rounded border border-red-500 bg-red-100 px-2 py-1.5">
    <!-- Header -->
    <div class="flex flex-row">
      <!-- Title -->
      <span class="max-w-60 truncate font-medium">{{ error.title ?? "Error" }}</span>
      <!-- Details -->
      <div class="ml-auto flex-shrink-0 pl-4 text-gray-700">
        <span>{{ toCasing(ErrorKind[error.kind], Casing.CAMEL, true) }}</span>
        <template v-if="error.type">
          / <span>{{ toCasing(ErrorType[error.type], Casing.CAMEL, true) }}</span></template
        >
      </div>
    </div>
    <!-- Text -->
    <div
      v-if="error.text?.lines.some((line) => line.spans.length > 0 && line.spans[0].content?.trim() != '')"
      class="mt-1 max-h-[190px] overflow-y-auto truncate"
    >
      <Text id="text" :model-value="error.text" is-minimal />
    </div>
    <div v-else class="mt-1">
      <span class="text-gray-900">No error message.</span>
    </div>
    <!-- Traceback -->
    <!-- ... -->
  </div>
</template>
