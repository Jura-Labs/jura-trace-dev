<script lang="ts">
  // ── InspectionChecklist ─────────────────────────────────────────────────
  // Interactive manual inspection checklist for visual assessment of images.
  // Each item has a pass/fail toggle and a collapsible hint explaining what
  // to look for. Designed for use alongside automated forensic results.

  // ── Types ───────────────────────────────────────────────────────────────

  type ItemState = 'unchecked' | 'pass' | 'fail';

  interface ChecklistItem {
    id: string;
    label: string;
    hint: string;
  }

  // ── Data ────────────────────────────────────────────────────────────────

  const ITEMS: ChecklistItem[] = [
    {
      id: 'text_rendering',
      label: 'Text rendering',
      hint: 'Are text elements clear, correctly spelled, and properly aligned? AI-generated images frequently produce garbled, misspelled, or nonsensical text — look for floating letters, incorrect words, or blended characters.',
    },
    {
      id: 'hand_anatomy',
      label: 'Hand and finger anatomy',
      hint: 'Do hands have the correct number of fingers with natural proportions? AI image generators commonly produce hands with extra, fused, or missing fingers, and joints that bend at implausible angles.',
    },
    {
      id: 'skin_texture',
      label: 'Skin texture',
      hint: 'Does skin show natural pores, lines, and imperfections? Synthetically generated skin often appears unnaturally smooth, overly blended, or symmetrical in ways real skin is not.',
    },
    {
      id: 'geometric_perspective',
      label: 'Geometric perspective',
      hint: 'Do parallel lines converge naturally? Is perspective consistent across the entire scene? Look for objects that appear to be at incompatible scales, or architecture whose geometry does not hold up under scrutiny.',
    },
    {
      id: 'shadow_direction',
      label: 'Shadow direction',
      hint: 'Are shadows consistent across all objects in the scene? Shadows cast by different objects should all originate from the same light source. Inconsistent shadow directions are a common sign of compositing.',
    },
    {
      id: 'reflections',
      label: 'Reflections',
      hint: 'Are reflections present where expected and physically correct? Reflective surfaces — water, glass, metal — should mirror their surroundings accurately. Missing or distorted reflections suggest manipulation or generation.',
    },
    {
      id: 'background_detail',
      label: 'Background detail',
      hint: 'Are background elements sharp and free of blending artefacts or repeating patterns? AI-generated backgrounds often contain copy-move artefacts, mismatched detail levels, or implausible geometry at the edges of objects.',
    },
    {
      id: 'contextual_logic',
      label: 'Contextual logic',
      hint: 'Does the scene make logical sense overall? Consider whether the combination of objects, people, environment, and lighting is physically plausible and contextually coherent.',
    },
  ];

  // ── State ───────────────────────────────────────────────────────────────

  /** Per-item state: unchecked, pass, or fail */
  let itemStates = $state<Record<string, ItemState>>(
    Object.fromEntries(ITEMS.map((item) => [item.id, 'unchecked' as ItemState]))
  );

  /** Which item's hint is currently expanded */
  let expandedHint = $state<string | null>(null);

  /** Whether the entire panel is open */
  let panelOpen = $state(false);

  // ── Derived ─────────────────────────────────────────────────────────────

  const checkedCount = $derived(
    Object.values(itemStates).filter((s) => s !== 'unchecked').length
  );

  const failCount = $derived(
    Object.values(itemStates).filter((s) => s === 'fail').length
  );

  const passCount = $derived(
    Object.values(itemStates).filter((s) => s === 'pass').length
  );

  const allChecked = $derived(checkedCount === ITEMS.length);

  // ── Handlers ────────────────────────────────────────────────────────────

  function cycleState(id: string) {
    const current = itemStates[id];
    const next: ItemState =
      current === 'unchecked' ? 'pass'
      : current === 'pass' ? 'fail'
      : 'unchecked';
    itemStates = { ...itemStates, [id]: next };
  }

  function toggleHint(id: string) {
    expandedHint = expandedHint === id ? null : id;
  }

  function resetAll() {
    itemStates = Object.fromEntries(ITEMS.map((item) => [item.id, 'unchecked' as ItemState]));
    expandedHint = null;
  }
</script>

<!--
  InspectionChecklist — collapsible manual visual inspection panel.

  Each row has:
    [state toggle]  [label]  [hint button]
    [hint text — conditionally visible]

  State cycles: unchecked → pass → fail → unchecked
-->
<div class="bg-obsidian/50 border border-graphite rounded-lg overflow-hidden">

  <!-- ── Panel header / toggle ──────────────────────────────────────── -->
  <button
    class="w-full flex items-center justify-between gap-3 px-4 py-3 text-left
           hover:bg-graphite/40 transition-colors duration-150
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
           focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
    onclick={() => { panelOpen = !panelOpen; }}
    aria-expanded={panelOpen}
    aria-controls="inspection-checklist-body"
  >
    <div class="flex items-center gap-2.5 min-w-0">
      <span class="text-sm font-medium text-flint dark:text-flint-light">Visual Inspection Checklist</span>

      <!-- Progress summary badges — shown when any items are checked -->
      {#if checkedCount > 0}
        <span class="flex items-center gap-1.5" aria-label="{passCount} passed, {failCount} flagged">
          {#if passCount > 0}
            <span
              class="text-xs px-1.5 py-px rounded bg-malachite/15 text-malachite dark:text-malachite-light border border-malachite/20"
              aria-hidden="true"
            >
              {passCount} pass
            </span>
          {/if}
          {#if failCount > 0}
            <span
              class="text-xs px-1.5 py-px rounded bg-cinnabar/15 text-cinnabar dark:text-cinnabar-light border border-cinnabar/20"
              aria-hidden="true"
            >
              {failCount} flag
            </span>
          {/if}
        </span>
      {:else}
        <span class="text-xs text-flint/50">{ITEMS.length} checks</span>
      {/if}
    </div>

    <!-- Chevron -->
    <svg
      class="w-4 h-4 text-flint dark:text-flint-light flex-shrink-0 transition-transform duration-200 motion-safe:{panelOpen ? 'rotate-180' : ''}"
      class:rotate-180={panelOpen}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  <!-- ── Panel body ────────────────────────────────────────────────── -->
  {#if panelOpen}
    <div
      id="inspection-checklist-body"
      class="border-t border-graphite"
    >

      <!-- Instruction row -->
      <div class="px-4 py-2.5 bg-lapis/5 border-b border-graphite/60 flex items-center justify-between gap-4">
        <p class="text-xs text-flint dark:text-flint-light leading-relaxed">
          Examine each element visually. Click a row to cycle through
          <span class="text-malachite dark:text-malachite-light">pass</span> /
          <span class="text-cinnabar dark:text-cinnabar-light">flag</span> / unchecked.
          Use the
          <span
            class="inline-flex items-center justify-center w-4 h-4 text-xs rounded-full
                   border border-lapis/40 text-lapis dark:text-lapis-light font-medium align-middle"
            aria-hidden="true"
          >?</span>
          button to show guidance for each check.
        </p>

        <!-- Reset link — only visible when something is checked -->
        {#if checkedCount > 0}
          <button
            class="flex-shrink-0 text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1"
            onclick={resetAll}
            aria-label="Reset all checklist items to unchecked"
          >
            Reset
          </button>
        {/if}
      </div>

      <!-- Item list -->
      <ul
        class="divide-y divide-graphite/60"
        role="list"
        aria-label="Visual inspection checklist items"
      >
        {#each ITEMS as item (item.id)}
          {@const state = itemStates[item.id]}
          <li class="group">
            <!-- Main row -->
            <div class="flex items-center gap-3 px-4 py-3">

              <!-- State toggle button — cycles unchecked → pass → fail -->
              <button
                class="flex-shrink-0 w-6 h-6 rounded flex items-center justify-center
                       transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-1 focus-visible:ring-offset-obsidian
                       {state === 'pass'
                         ? 'bg-malachite/20 border border-malachite/40 hover:bg-malachite/30'
                         : state === 'fail'
                           ? 'bg-cinnabar/20 border border-cinnabar/40 hover:bg-cinnabar/30'
                           : 'bg-graphite border border-graphite-light hover:border-flint/50'}"
                onclick={() => cycleState(item.id)}
                aria-label="{item.label}: {state === 'pass' ? 'passed — click to flag' : state === 'fail' ? 'flagged — click to clear' : 'unchecked — click to mark as passed'}"
                aria-pressed={state !== 'unchecked'}
              >
                {#if state === 'pass'}
                  <!-- Tick mark -->
                  <svg class="w-3.5 h-3.5 text-malachite dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
                  </svg>
                {:else if state === 'fail'}
                  <!-- Flag / X mark -->
                  <svg class="w-3.5 h-3.5 text-cinnabar dark:text-cinnabar-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                {:else}
                  <!-- Empty circle placeholder -->
                  <span class="w-2 h-2 rounded-full border border-flint/30" aria-hidden="true"></span>
                {/if}
              </button>

              <!-- Label -->
              <span
                class="flex-1 text-sm cursor-pointer select-none
                       {state === 'pass' ? 'text-malachite-light' : state === 'fail' ? 'text-cinnabar-light' : 'text-quartz'}"
                onclick={() => cycleState(item.id)}
                role="presentation"
              >
                {item.label}
              </span>

              <!-- Hint toggle button -->
              <button
                class="flex-shrink-0 w-5 h-5 rounded-full text-xs font-medium
                       flex items-center justify-center transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-1 focus-visible:ring-offset-obsidian
                       {expandedHint === item.id
                         ? 'bg-lapis/20 border border-lapis/40 text-lapis dark:text-lapis-light'
                         : 'border border-lapis/30 text-lapis/60 dark:text-lapis-light/60 hover:border-lapis/60 hover:text-lapis dark:hover:text-lapis-light'}"
                onclick={() => toggleHint(item.id)}
                aria-label="{expandedHint === item.id ? 'Hide' : 'Show'} guidance for {item.label}"
                aria-expanded={expandedHint === item.id}
                aria-controls="hint-{item.id}"
              >
                ?
              </button>
            </div>

            <!-- Hint text — conditionally visible -->
            {#if expandedHint === item.id}
              <div
                id="hint-{item.id}"
                class="px-4 pb-3 pt-0 ml-9"
                role="region"
                aria-label="Guidance for {item.label}"
              >
                <p class="text-xs text-lapis/80 leading-relaxed bg-lapis/5 border border-lapis/15 rounded-md px-3 py-2">
                  {item.hint}
                </p>
              </div>
            {/if}
          </li>
        {/each}
      </ul>

      <!-- Footer summary — shown when all items are reviewed -->
      {#if allChecked}
        <div
          class="px-4 py-3 border-t border-graphite bg-graphite/30 flex items-center justify-between gap-4"
          role="status"
          aria-live="polite"
          aria-label="Checklist complete: {passCount} items passed, {failCount} items flagged"
        >
          <div class="flex items-center gap-3 text-xs">
            <span class="text-flint dark:text-flint-light">Checklist complete —</span>
            {#if failCount === 0}
              <span class="text-malachite dark:text-malachite-light font-medium">All {passCount} items passed</span>
            {:else if failCount <= 2}
              <span class="text-amber dark:text-amber-light font-medium">{failCount} concern{failCount === 1 ? '' : 's'} flagged</span>
            {:else}
              <span class="text-cinnabar dark:text-cinnabar-light font-medium">{failCount} concerns flagged</span>
            {/if}
          </div>
          <button
            class="text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1"
            onclick={resetAll}
            aria-label="Reset checklist"
          >
            Reset
          </button>
        </div>
      {/if}

    </div>
  {/if}

</div>
