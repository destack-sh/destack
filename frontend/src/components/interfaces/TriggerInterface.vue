<script lang="ts" setup>
import { TriggerType, type Trigger } from "@/gql/graphql";
import { Switch } from "@headlessui/vue";

const props = defineProps<{ modelValue: Trigger; readonly: boolean }>();
const emit = defineEmits<{ (e: "update:modelValue", value: Trigger): void }>();
</script>
<template>
  <div ref="containerRef" class="relative">
    <!-- Type & active -->
    <Switch :model-value="props.modelValue.active" />
    <!-- Trigger details -->
    <div v-if="props.modelValue.type == TriggerType.Time" class="flex flex-col">
      <!-- Interval or cron -->
      <div class="flex flex-row">
        <button>Interval</button>
        <button>Cron</button>
      </div>
      <!-- Interval -->
      <!-- Cron -->
    </div>
    <div v-else>
      <!-- show error? -->
    </div>
  </div>
</template>
