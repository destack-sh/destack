<script lang="ts" setup>
import type { Conditional } from "@/gql/graphql";
import type { Field } from "@/state/module";

const props = defineProps<{ field: Field; modelValue?: Conditional }>();
const emit = defineEmits<{ (e: "update:modelValue", value?: Conditional): void }>();
</script>
<template>
  <div>
    <!-- nocheckin: conditional -->
    filter {{ field.name }}
  </div>
</template>
