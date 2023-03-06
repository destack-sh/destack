<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/notifications/NotificationArea.vue";
import { useValidName, useValidSlug } from "@/composables/useValidation";
import { useRedirectIfNotLoggedIn } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { useTitle } from "@vueuse/core";
import { computed, onMounted, ref, type Ref } from "vue";
import { useRouter } from "vue-router";

const title = useTitle();
title.value = "Start your organization";

useRedirectIfNotLoggedIn();

useRedirectIfNotLoggedIn({ name: "Signup" });

const name: Ref<string> = ref("Hooli, Inc.");
const slug: Ref<string> = ref("hooli");
const slugModified = ref(false);

function syncSlugIfUnmodified() {
  if (!slugModified.value) {
    let nameTrimmed = name.value.toLowerCase();
    if (nameTrimmed.includes(",")) {
      nameTrimmed = nameTrimmed.split(",")[0];
    }
    if (nameTrimmed.includes("-")) {
      nameTrimmed = nameTrimmed.split("-")[0];
    }
    slug.value = nameTrimmed.replace(/[^a-z0-9_-]/g, "-");
  }
}

const validName = useValidName(name);
const validSlug = useValidSlug(slug);

const canComplete = computed(
  () =>
    !creating.value &&
    !validSlug.loading.value &&
    validSlug.available.value &&
    validSlug.valid.value &&
    validName.valid.value
);

// auto-focus name on load
const nameRef: Ref<HTMLInputElement | null> = ref(null);
onMounted(() => {
  nameRef.value?.focus();
  // select all
  nameRef.value?.setSelectionRange(0, nameRef.value?.value?.length ?? 0);
});

const creating: Ref<boolean> = ref(false);

const ops = useOperations();
const router = useRouter();
const notifications = useNotifications();
async function createOrganization() {
  creating.value = true;
  const create = await ops.organization.create(name.value, slug.value);
  creating.value = false;
  if (create?.data?.createOrganization.__typename == "Organization") {
    router.push(`/settings/${slug.value}#members`);
    notifications.show({
      kind: "success",
      type: "organization.created",
      message: "Organization created",
      description: `It's official: ${name.value} is ready to take over the world.`,
    });
  }
}
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileButton />
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="-mx-32 mt-4 text-5xl font-bold">Start your organization</h1>
      <p class="-mx-4 mt-4 text-lg text-orange-700">AI is yours for the making - together.</p>
      <!-- <p class="mt-1 text-sm text-gray-700">(Think big: not just a single feature/task)</p> -->
      <!-- Fields to complete -->
      <div class="mt-10 flex flex-col gap-4">
        <!-- TODO @Incomplete: select owner -->

        <!-- Full name -->
        <div class="text-left">
          <span class="text-md text-gray-700">Bench name</span>
          <input
            ref="nameRef"
            type="text"
            minlength="3"
            maxlength="128"
            v-model="name"
            @input="syncSlugIfUnmodified"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
            spellcheck="false"
          />
          <FadeTransition mode="out-in">
            <span class="mt-1 text-sm text-yellow-500" v-if="!validName.valid.value"
              >The bots don't like this name.</span
            >
            <span class="mt-1 text-sm text-gray-500" v-else>Great name.</span>
          </FadeTransition>
        </div>

        <!-- slug -->
        <div class="text-left">
          <span class="text-md text-gray-700">A robot-friendly name</span>
          <input
            type="text"
            minlength="3"
            maxlength="128"
            pattern="[a-z0-9_-]+"
            :value="slug"
            @input="(event) => ((slug = event.target?.value), (slugModified = true))"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
            spellcheck="false"
          />
          <FadeTransition mode="out-in">
            <span v-if="!validSlug.valid.value" class="mt-1 text-sm text-orange-600">
              The bots want a name like <span class="font-mono text-xs text-gray-500">[a-z0-9_-]{3,}</span>
            </span>
            <span v-else-if="validSlug.loading.value" class="mt-1">&nbsp;</span>
            <span v-else-if="!validSlug.available.value" class="mt-1 text-sm text-red-600">
              That name is
              <router-link
                :to="`/${slug}`"
                class="underline decoration-dotted underline-offset-2 hover:decoration-solid focus:decoration-solid focus:outline-none"
                >taken</router-link
              >.
            </span>
            <span v-else class="mt-1 text-sm text-gray-500">Yours for the taking.</span>
          </FadeTransition>
        </div>

        <button
          :disabled="!canComplete"
          class="mt-6 w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          :class="{ 'pointer-events-none opacity-50': !canComplete }"
          @click="createOrganization"
        >
          Start &rarr;
        </button>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
