<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OwnerSelect from "@/components/basic/OwnerSelect.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/notifications/NotificationArea.vue";
import { graphql } from "@/gql";
import { ProjectType, ProjectVisibility } from "@/gql/graphql";
import { useAuth, useRedirectIfNotLoggedIn } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { GlobeAltIcon, LockClosedIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed, onMounted, ref, watchEffect, type Ref } from "vue";
import { useRouter } from "vue-router";

const title = useTitle();
title.value = "Create your Bench";

useRedirectIfNotLoggedIn();

const auth = useAuth();
const owner: Ref<{ id: string; name: string; slug: string } | null> = ref(null);
const name: Ref<string> = ref("Sandbox");
const slug: Ref<string> = ref("sandbox");
const slugModified = ref(false);
const visibility: Ref<ProjectVisibility> = ref(ProjectVisibility.Private);
const type: Ref<ProjectType> = ref(ProjectType.Executable);

const visibilities = computed(() => [
  {
    value: ProjectVisibility.Private,
    label: "Private",
    icon: LockClosedIcon,
    description: `Only ${owner.value?.id == auth.me.value?.id ? "you" : owner.value?.name} can see this Bench`,
  },
  {
    value: ProjectVisibility.Public,
    label: "Public",
    icon: GlobeAltIcon,
    description: `Everyone can see this Bench`,
  },
]);

// set owner to self as soon as auth is set
watchEffect(() => {
  if (auth.me.value != null && owner.value == null) {
    owner.value = auth.me.value;
  }
});

function syncSlugIfUnmodified() {
  if (!slugModified.value) {
    slug.value = name.value.toLowerCase().replace(/[^a-z0-9_-]/g, "-");
  }
}

const isValidName = computed(() => (name.value?.length ?? 0) >= 2);
const isValidSlug = computed(() => /^[a-z0-9_-]{3,}$/.test(slug.value ?? "") && (slug.value?.length ?? 0 >= 4));

// check if project slug is available
const { result: existingProject, loading: existingProjectLoading } = useQuery(
  graphql(/* GraphQL */ `
    query existingProjectBySlug($owner: String!, $project: String!) {
      projectBySlug(owner: $owner, project: $project) {
        id
        slug
      }
    }
  `),
  computed(() => ({
    owner: owner.value?.slug,
    project: slug.value,
  })) as any,
  // we don't want to cache this to (almost) guarantee that the slug is valid,
  // and to definitely re-fetch projectBySlug when a slug is created/changed
  { fetchPolicy: "no-cache" }
);
const isAvailableSlug = computed(() => existingProject.value?.projectBySlug == null);
const canComplete = computed(
  () =>
    !creating.value && !existingProjectLoading.value && isValidName.value && isValidSlug.value && isAvailableSlug.value
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
async function createProject() {
  creating.value = true;
  const create = await ops.project.create(
    owner.value?.id as string,
    name.value,
    slug.value,
    type.value,
    visibility.value
  );
  creating.value = false;
  if (create?.data?.createProject.__typename == "Project") {
    router.push(`/${owner.value?.slug}/${slug.value}`);
    notifications.show({
      kind: "success",
      type: "project.created",
      message: "Bench crafted",
      description: "Your brand new Bench is ready for work.",
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
    <div class="mx-auto mt-20 w-96 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="-mx-2 mt-4 text-5xl font-bold">Create your Bench</h1>
      <p class="mx-4 mt-4 text-lg text-orange-700">AI instruction, evaluation and deployment.</p>
      <p class="mt-1 text-sm text-gray-700">(Think big: a product/team, not a single feature/task)</p>
      <!-- Fields to complete -->
      <div class="mt-10 flex flex-col gap-4">
        <!-- Full name & visibility -->
        <div class="flex w-full flex-col text-left">
          <!-- Title -->
          <span class="text-md text-gray-700">Bench name</span>
          <div class="flex flex-row">
            <!-- Name -->
            <input
              ref="nameRef"
              type="text"
              minlength="3"
              maxlength="128"
              v-model="name"
              @input="syncSlugIfUnmodified"
              class="mt-1 w-full rounded-sm rounded-r-none border border-r-0 border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
              spellcheck="false"
            />
            <!-- Visbility -->
            <Listbox
              :model-value="visibility"
              @update:model-value="visibility = $event.value"
              as="div"
              class="relative"
            >
              <ListboxButton
                class="mt-1 rounded-l-none border border-l-0 border-orange-600 py-1.5 px-2 hover:bg-orange-50 focus:bg-orange-50 focus:outline-none"
              >
                <span>
                  <component
                    :is="visibilities.find((v) => v.value == visibility)?.icon"
                    class="h-5 w-5 text-gray-700"
                  />
                </span>
              </ListboxButton>
              <FadeTransition>
                <ListboxOptions
                  class="absolute left-0 top-8 z-10 mt-0 max-h-64 w-56 overflow-y-auto rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
                >
                  <ListboxOption
                    v-for="vis in visibilities"
                    :key="vis.value"
                    :value="vis"
                    v-slot="{ active, selected }"
                    as="template"
                  >
                    <div
                      class="flex flex-row items-center gap-3 text-left hover:cursor-pointer"
                      :class="[
                        active ? 'bg-orange-50' : '',
                        'block py-1.5 px-2 text-sm text-gray-900',
                        selected ? 'text-orange-600' : '',
                      ]"
                    >
                      <span class="flex flex-col">
                        <span>{{ vis.label }}</span>
                        <span class="text-xs text-gray-500">{{ vis.description }}</span>
                      </span>
                    </div>
                  </ListboxOption>
                </ListboxOptions>
              </FadeTransition>
            </Listbox>
          </div>
          <!-- Validation message -->
          <div class="text-left">
            <FadeTransition mode="out-in">
              <span class="mt-1 text-sm text-yellow-600" v-if="!isValidName">That's not a name we can print.</span>
              <span class="mt-1 text-sm text-gray-500" v-else>Great name.</span>
            </FadeTransition>
          </div>
        </div>

        <!-- Owner & slug -->
        <div class="flex w-full flex-col">
          <!-- Title -->
          <span class="text-md text-left text-gray-700">Bench location</span>
          <!-- Owner & slug -->
          <div class="flex w-full flex-row">
            <OwnerSelect v-model="owner">
              <template v-slot:button="{ open }">
                <ListboxButton
                  class="mt-1 flex flex-row items-center gap-1 rounded-sm rounded-r-none border border-orange-600 px-3 py-1 py-1 hover:bg-orange-50 focus:bg-orange-50 focus:outline-none"
                  :class="open ? 'bg-orange-50' : ''"
                >
                  <p class="">{{ owner?.slug }}</p>
                  <span class="text-gray-500">/</span>
                </ListboxButton>
              </template>
            </OwnerSelect>
            <input
              type="text"
              minlength="3"
              maxlength="128"
              pattern="[a-z0-9_-]+"
              :value="slug"
              @input="(event) => ((slug = event.target?.value), (slugModified = true))"
              class="mt-1 w-full rounded-sm rounded-l-none border border-l-0 border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
              spellcheck="false"
            />
          </div>
          <!-- Validation message (below both) -->
          <div class="text-left">
            <FadeTransition mode="out-in">
              <span v-if="!isValidSlug" class="mt-1 text-sm text-orange-600">
                The bots want something like <span class="font-mono text-xs text-gray-500">[a-z0-9_-]{3,}</span>
              </span>
              <span v-else-if="existingProjectLoading" class="mt-1">&nbsp;</span>
              <span v-else-if="!isAvailableSlug" class="mt-1 text-sm text-red-600">
                That location is
                <router-link
                  :to="`/${owner?.slug}/${slug}`"
                  class="underline decoration-dotted underline-offset-2 hover:decoration-solid focus:decoration-solid focus:outline-none"
                  >taken</router-link
                >.
              </span>
              <span v-else class="mt-1 text-sm text-gray-500">Yours for the taking.</span>
            </FadeTransition>
          </div>
        </div>

        <button
          :disabled="!canComplete"
          class="mt-6 w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          :class="{ 'pointer-events-none opacity-50': !canComplete }"
          @click="createProject"
        >
          Build &rarr;
        </button>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
