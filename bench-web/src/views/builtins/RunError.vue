<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { ErrorKind, ErrorType, type RunData, type ErrorData } from "@/proto/wire";
import Text from "@/views/content/Text.vue";

const props = defineProps<{ run?: RunData; error: ErrorData }>();
</script>
<template>
  <div class="">
    <!-- Header -->
    <div class="flex flex-row">
      <!-- Title -->
      <span class="max-w-60 truncate font-medium">{{ error.title ?? "Error" }}</span>
      <!-- Details -->
      <div class="ml-auto flex-shrink-0 pl-4 text-gray-400">
        <span>{{ toCamelName(ErrorKind, error.kind) }}</span>
        <template v-if="error.type">
          / <span>{{ toCamelName(ErrorType, error.type) }}</span></template
        >
      </div>
    </div>
    <!-- Text -->
    <div v-if="error.text" class="mt-1 max-h-[190px] overflow-y-auto truncate">
      <Text id="text" :model-value="error.text" is-minimal />
    </div>
    <div v-else class="mt-1">
      <span class="text-gray-400">No Error Message</span>
    </div>
    <!-- Traceback -->
    <!-- ... -->
  </div>
</template>
