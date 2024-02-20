<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  clientId: string;
  user: { username: string; name: string };
}>();

const INITIALS_LENGTH = 2;
const initials = computed(() => {
  const name = props.user.name ?? props.user.username;
  const parts = name.split(" ");
  if (parts.length === 1) {
    return name.substring(0, INITIALS_LENGTH);
  } else {
    return parts[0].substring(0, 1) + parts[parts.length - 1].substring(0, 1);
  }
});
</script>

<template>
  <div
    class="relative justify-center rounded-2xl border border-orange-900/[20%] bg-orange-100 text-center align-middle text-sm"
  >
    <!-- initials at exact center -->
    <span class="absolute inset-0 mb-0.5 flex items-center justify-center uppercase">
      {{ initials }}
    </span>
  </div>
</template>
