<script lang="ts">
  // No reactive state needed — this is a static help page.
  // Section IDs are used for deep linking from contextual help buttons.
</script>

<!-- ── Page header ────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Application Settings
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    The Settings page lets you configure Ollama for AI-assisted analysis, manage named
    deployment profiles, check service health, choose where your local database is stored,
    review your licence tier, and review licence and version information.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- ── Table of contents ───────────────────────────────────────────── -->
<nav aria-label="Page contents" class="mb-10">
  <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-3">
    On this page
  </p>
  <ol class="space-y-1 text-sm">
    <li>
      <a href="#ollama"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. Ollama Configuration
      </a>
    </li>
    <li>
      <a href="#deployment-profiles"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. Deployment Profiles
      </a>
    </li>
    <li>
      <a href="#service-status"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. Service Status
      </a>
    </li>
    <li>
      <a href="#database"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Database Location
      </a>
    </li>
    <li>
      <a href="#about"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        5. About
      </a>
    </li>
    <li>
      <a href="#setup-wizard"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        6. Setup Wizard
      </a>
    </li>
    <li>
      <a href="#auto-updater"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        7. Updates
      </a>
    </li>
    <li>
      <a href="#your-plan"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        8. Your Plan
      </a>
    </li>
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. Ollama Configuration
     ══════════════════════════════════════════════════════════════════ -->
<section id="ollama" class="mb-12" aria-labelledby="heading-ollama">
  <h2 id="heading-ollama" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    1. Ollama Configuration
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Ollama is a free tool that runs AI models locally on your computer. In Jura Trace,
    it powers two optional features that sit on top of the core forensic pipeline —
    neither replaces any of the 21 standard detectors.
  </p>

  <!-- ── What Ollama does ─────────────────────────────────────────── -->
  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What Ollama does
  </h3>

  <div class="space-y-5 mb-6">

    <!-- Feature 1: Read Text from Images -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Feature 1 — Read Text from Images
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        When you verify an image that contains visible text — a screenshot, a meme, a social
        media post, a photograph of a sign or document — a <strong class="text-text-light dark:text-text-dark">Read Text (Ollama)</strong>
        button appears in the verification results. Click it and the AI reads and transcribes
        all text visible in the image.
      </p>
      <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-2">Useful for</p>
      <ul class="space-y-1 text-sm text-flint dark:text-flint-light">
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Extracting text from screenshots of social media posts</li>
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Reading watermarks and captions embedded in images</li>
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Identifying text in photographs of signs, documents, or posters</li>
      </ul>
    </div>

    <!-- Feature 2: Claim Verification -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Feature 2 — Claim Verification
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        When you verify a video or audio file containing speech, Jura Trace transcribes
        what was said. If Ollama is running, it then checks the factual claims in that
        transcript against a knowledge base and returns a verdict for each claim:
        <strong class="text-text-light dark:text-text-dark">supported</strong>,
        <strong class="text-text-light dark:text-text-dark">disputed</strong>, or
        <strong class="text-text-light dark:text-text-dark">unverifiable</strong>.
      </p>
      <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-2">Useful for</p>
      <ul class="space-y-1 text-sm text-flint dark:text-flint-light">
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Fact-checking video interviews or speeches</li>
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Verifying claims made in news footage</li>
        <li class="flex gap-2"><span class="text-lapis dark:text-lapis-light flex-none">→</span> Checking statements in audio recordings</li>
      </ul>
    </div>

  </div>

  <!-- ── What works without Ollama ───────────────────────────────── -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4 mb-8">
    <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">Everything else works without Ollama</p>
    <ul class="space-y-1 text-sm text-flint dark:text-flint-light">
      <li class="flex gap-2"><span class="text-malachite dark:text-malachite-light flex-none">✓</span> All 21 forensic detectors (ELA, noise, copy-move, deepfake, and more)</li>
      <li class="flex gap-2"><span class="text-malachite dark:text-malachite-light flex-none">✓</span> C2PA provenance verification</li>
      <li class="flex gap-2"><span class="text-malachite dark:text-malachite-light flex-none">✓</span> Invisible watermark embedding and detection</li>
      <li class="flex gap-2"><span class="text-malachite dark:text-malachite-light flex-none">✓</span> EXIF anomaly analysis</li>
      <li class="flex gap-2"><span class="text-malachite dark:text-malachite-light flex-none">✓</span> Video and audio metadata (requires FFmpeg)</li>
    </ul>
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mt-3">
      Ollama adds intelligence on top of the forensic pipeline. It does not replace any core feature.
    </p>
  </div>

  <!-- ── How to test Ollama ───────────────────────────────────────── -->
  <h3 id="ollama-testing" class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    How to test Ollama
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Follow these steps to confirm Ollama is set up and working in Jura Trace.
  </p>

  <div class="space-y-4 mb-8">

    <!-- Step 1 -->
    <div class="flex gap-4">
      <span class="flex-none w-7 h-7 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center mt-0.5">1</span>
      <div>
        <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">Confirm Ollama is running</p>
        <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
          Go to <strong class="text-text-light dark:text-text-dark">Settings → Service Status</strong>.
          The Ollama row should show <strong class="text-malachite dark:text-malachite-light">Online</strong>.
          If it shows Unavailable, open the Ollama application — on macOS look for its icon in the menu bar;
          on Windows look in the system tray.
        </p>
      </div>
    </div>

    <!-- Step 2 -->
    <div class="flex gap-4">
      <span class="flex-none w-7 h-7 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center mt-0.5">2</span>
      <div>
        <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">Test text reading</p>
        <ol class="space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed list-none">
          <li>a. Take a screenshot of a social media post or a news article.</li>
          <li>b. Go to <strong class="text-text-light dark:text-text-dark">VERIFY</strong> and drop the screenshot onto the drop zone.</li>
          <li>c. Wait for analysis to complete.</li>
          <li>d. Scroll down in the results and find the <strong class="text-text-light dark:text-text-dark">Read Text (Ollama)</strong> button.</li>
          <li>e. Click it. The AI reads the image and the extracted text appears in a box below the button.</li>
        </ol>
      </div>
    </div>

    <!-- Step 3 -->
    <div class="flex gap-4">
      <span class="flex-none w-7 h-7 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center mt-0.5">3</span>
      <div>
        <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">Test claim checking</p>
        <ol class="space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed list-none">
          <li>a. Find a short video clip with someone speaking — a news clip or interview works well.</li>
          <li>b. Go to <strong class="text-text-light dark:text-text-dark">VERIFY</strong> and drop the video onto the drop zone.</li>
          <li>c. Select <strong class="text-text-light dark:text-text-dark">Deep</strong> mode at the top of the page (this enables transcription).</li>
          <li>d. Wait for analysis — video analysis can take 30–60 seconds.</li>
          <li>e. If speech was detected, the transcript appears in the results panel.</li>
          <li>f. If Ollama is running, each factual claim in the transcript is shown with a verdict: <strong class="text-text-light dark:text-text-dark">Supported</strong>, <strong class="text-text-light dark:text-text-dark">Disputed</strong>, or <strong class="text-text-light dark:text-text-dark">Unverifiable</strong>.</li>
        </ol>
      </div>
    </div>

  </div>

  <!-- ── Installing Ollama ────────────────────────────────────────── -->
  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Installing Ollama
  </h3>

  <ol class="space-y-4 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Download and install Ollama from
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">ollama.com</span>.
        Ollama runs as a background service on macOS, Windows, and Linux.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Pull the required models. In a terminal:
        <br />
        <code class="block mt-2 font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-3 py-2 rounded leading-relaxed">
          ollama pull llava:7b<br />
          ollama pull qwen2.5:7b-instruct
        </code>
        Each model is approximately 4–5 GB. They are stored locally and never transmitted externally.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        Verify the models are installed. In a terminal:
        <br />
        <code class="block mt-2 font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-3 py-2 rounded">ollama list</code>
        You should see both
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">llava:7b</span>
        and
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">qwen2.5:7b-instruct</span>
        listed.
      </span>
    </li>
  </ol>

  <!-- ── Settings fields ─────────────────────────────────────────── -->
  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Settings fields
  </h3>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Field</th>
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Default</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Notes</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Ollama URL</td>
          <td class="py-2 pr-4 font-mono text-xs">http://localhost:11434</td>
          <td class="py-2">Change this if Ollama is running on a different port, or if you are pointing to a shared institutional server on your local network.</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Vision model</td>
          <td class="py-2 pr-4 font-mono text-xs">llava:7b</td>
          <td class="py-2">Used for image text reading and AI descriptions. Must be a LLaVA-compatible model tag installed in your local Ollama instance.</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Text model</td>
          <td class="py-2 pr-4 font-mono text-xs">qwen2.5:7b-instruct</td>
          <td class="py-2">Used for claim verification. Must be an instruction-tuned text model installed in your local Ollama instance.</td>
        </tr>
      </tbody>
    </table>
  </div>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
    Settings are saved locally on your device when you click
    <strong class="text-text-light dark:text-text-dark">Save settings</strong>.
    They persist across application restarts. Use
    <a href="#deployment-profiles" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis">Deployment Profiles</a>
    to save and switch between multiple named configurations.
  </p>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. Deployment Profiles
     ══════════════════════════════════════════════════════════════════ -->
<section id="deployment-profiles" class="mb-12" aria-labelledby="heading-profiles">
  <h2 id="heading-profiles" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    2. Deployment Profiles
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Deployment profiles let you save your current Ollama configuration under a name and
    switch between saved configurations quickly. This is useful when you use Jura Trace
    in different contexts — for example, when working in an office with access to a
    shared institutional Ollama server versus working offline in the field.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Saving a profile
  </h3>

  <ol class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Configure the Ollama URL and model names to match the environment you want to save.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Click <strong class="text-text-light dark:text-text-dark">Save as profile</strong>
        in the Deployment Profiles section. Enter a name — for example,
        "Office" or "Field" — and confirm.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        The profile appears in the profile list with the date it was created. You can save
        up to ten profiles.
      </span>
    </li>
  </ol>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Loading a profile
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Click <strong class="text-text-light dark:text-text-dark">Load</strong> next to the
    profile you want to apply. The Ollama URL and model fields update immediately to
    reflect the saved values. A confirmation message appears briefly to confirm the
    profile has been loaded.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    After loading a profile, click
    <strong class="text-text-light dark:text-text-dark">Save settings</strong>
    to persist the loaded values to the application's local settings. If you do not save, the values will
    revert to the previous configuration on the next application restart.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Deleting a profile
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Click <strong class="text-text-light dark:text-text-dark">Delete</strong> next to a
    profile, then confirm the deletion. This removes the profile from the application's local settings.
    The active settings are not affected — only the saved profile entry is removed.
  </p>

  <!-- Callout: use case example -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Example.</strong>
      A photojournalist might save an "Office" profile pointing to a shared Ollama server
      at <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">http://192.168.1.10:11434</span>
      used by the editorial team, and a "Field" profile pointing to
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">http://localhost:11434</span>
      for their laptop's local instance. Switching between them takes a single click.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     3. Service Status
     ══════════════════════════════════════════════════════════════════ -->
<section id="service-status" class="mb-12" aria-labelledby="heading-service-status">
  <h2 id="heading-service-status" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    3. Service Status
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Service Status panel shows the current availability of the two optional services
    that extend Jura Trace's analysis capabilities. Both services are optional — the core
    protect and verify features work without them.
  </p>

  <div class="space-y-5 mb-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Analysis Services
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        This indicator shows the status of the Analysis Engine, which runs locally on
        port 8200. The Analysis Engine provides forensic image analysis (ELA, noise, copy-move,
        deepfake detection, JPEG ghost, regional analysis), watermark embed and extract,
        video and audio processing, transcription, and the on-demand investigation tools
        (NPR, shadow consistency, splice boundary) available in Expert View.
      </p>
      <div class="overflow-x-auto">
        <table class="w-full text-sm border-collapse">
          <thead>
            <tr class="border-b border-border-light dark:border-border-dark">
              <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Status</th>
              <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Meaning</th>
            </tr>
          </thead>
          <tbody class="text-flint dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-malachite dark:text-malachite-light">Online</td>
              <td class="py-2">The Analysis Engine is running and responding. Forensic analysis and watermarking are available.</td>
            </tr>
            <tr>
              <td class="py-2 pr-4 font-semibold text-cinnabar dark:text-cinnabar-light">Offline</td>
              <td class="py-2">The Analysis Engine is not running or is not reachable on port 8200. Forensic analysis will be skipped during verification. C2PA checking and EXIF analysis still work.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Ollama
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        This indicator shows whether Ollama is reachable at the URL configured in the
        Ollama Configuration section above.
      </p>
      <div class="overflow-x-auto">
        <table class="w-full text-sm border-collapse">
          <thead>
            <tr class="border-b border-border-light dark:border-border-dark">
              <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Status</th>
              <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Meaning</th>
            </tr>
          </thead>
          <tbody class="text-flint dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-malachite dark:text-malachite-light">Online</td>
              <td class="py-2">Ollama is running and at least one model is available. Tier 3 descriptions and RAG claim verification are enabled.</td>
            </tr>
            <tr>
              <td class="py-2 pr-4 font-semibold text-cinnabar dark:text-cinnabar-light">Unavailable</td>
              <td class="py-2">Ollama is not running, not reachable at the configured URL, or has no models installed. These features will be skipped; all other analysis continues normally.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What to do when a service is offline
  </h3>

  <div class="space-y-3 mb-4">

    <div>
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Analysis Services offline
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The Analysis Engine starts automatically when you launch Jura Trace. If it shows as
        <strong class="text-text-light dark:text-text-dark">Offline</strong>, try restarting
        the application. If the problem persists, check that no other process is using
        port 8200 on your device, then restart again. Once the service is running, click
        <strong class="text-text-light dark:text-text-dark">Refresh</strong> in the Service
        Status panel to confirm the connection.
      </p>
    </div>

    <div>
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Ollama unavailable
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Ensure Ollama is running (on macOS it runs as a menu bar application) and that
        the URL in the Ollama Configuration section matches the port Ollama is using.
        If you have not yet pulled the required models, refer to the
        <a href="#ollama" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis">installation steps</a>
        above.
      </p>
    </div>

  </div>

  <!-- Callout: refresh -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Tip:</strong>
      The Service Status panel does not update automatically. Click
      <strong class="text-text-light dark:text-text-dark">Refresh</strong>
      after starting a service to get the current status.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     4. Database Location
     ══════════════════════════════════════════════════════════════════ -->
<section id="database" class="mb-12" aria-labelledby="heading-database">
  <h2 id="heading-database" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    4. Database Location
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Jura Trace stores all asset records, fingerprints, verification history, and audit
    logs in a single SQLite database file named
    <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>.
    The current file path is shown at the top of the Database Location section.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Default locations
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
    Unless you have moved the database, it lives in the application's data directory for
    your platform:
  </p>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Platform</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Default path</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold">macOS</td>
          <td class="py-2 font-mono text-xs break-all">~/Library/Application Support/com.juralabs.jura-trace/jura_trace.db</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold">Windows</td>
          <td class="py-2 font-mono text-xs break-all">%APPDATA%\com.juralabs.jura-trace\jura_trace.db</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold">Linux</td>
          <td class="py-2 font-mono text-xs break-all">~/.local/share/com.juralabs.jura-trace/jura_trace.db</td>
        </tr>
      </tbody>
    </table>
  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Path resolution priority
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
    Jura Trace resolves the database path in the following order of precedence. The first
    valid path found is used:
  </p>

  <ol class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Environment variable</strong> —
        if the
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">JURA_DB_PATH</span>
        environment variable is set to a valid file path, that path is used. Useful for
        scripted or institutional deployments.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Config file</strong> —
        if a
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">config.json</span>
        file in the application data directory contains a
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">db_path</span>
        key pointing to a valid file, that path is used. This file is written
        automatically when you change the database location via the Settings page.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Default</strong> —
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>
        in the platform application data directory, as shown in the table above.
      </span>
    </li>
  </ol>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Moving the database
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    You may want to move the database to a different location — for example, to a shared
    network drive accessible to colleagues, or to a directory governed by your
    institution's backup policy.
  </p>

  <ol class="space-y-4 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Click <strong class="text-text-light dark:text-text-dark">Change location</strong>
        in the Database Location section.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Select the destination folder in the file picker. Jura Trace will use the name
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>
        in the chosen folder.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        Jura Trace copies the existing database to the new location, verifies the copy is
        intact, then updates
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">config.json</span>
        to point to the new path. The original file remains in place until the move is
        confirmed.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">4</span>
      <span>
        Restart the application for the new path to take full effect. Jura Trace will
        open the database at the new location on startup.
      </span>
    </li>
  </ol>

  <!-- Callout: backup -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Back up your database regularly.</strong>
      The
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>
      file contains your entire asset catalogue, fingerprint records, verification history,
      and audit log. It is a single file and can be copied to any backup destination
      whilst Jura Trace is closed.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     5. About
     ══════════════════════════════════════════════════════════════════ -->
<section id="about" class="mb-12" aria-labelledby="heading-about">
  <h2 id="heading-about" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    5. About
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    The About section displays the installed application version and licensing information.
  </p>

  <dl class="space-y-4">
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Application</dt>
      <dd class="mt-1 text-sm text-flint dark:text-flint-light leading-relaxed">Jura Trace</dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Developer</dt>
      <dd class="mt-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        Juralabs Community Interest Company (UK) —
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">juralabs.org</span>
      </dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Licence</dt>
      <dd class="mt-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        PolyForm Noncommercial 1.0.0. You may use, study, and adapt this software for
        non-commercial purposes. Commercial use requires a separate licence agreement.
        See the
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">LICENSE</span>
        file included with the application for the full licence terms.
      </dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Version</dt>
      <dd class="mt-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        The current version number is shown in the Settings page. To report a bug or
        request a feature, visit the project repository or contact
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">juralabs.org</span>.
      </dd>
    </div>
  </dl>

  <!-- Callout: local-first reminder -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4 mt-6">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Local-first by design.</strong>
      Jura Trace does not collect usage data, send telemetry, or connect to any cloud
      service. Every operation — protect, verify, analyse, store — runs entirely on your
      device. Your content and your audit trail remain yours.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     6. Setup Wizard
     ══════════════════════════════════════════════════════════════════ -->
<section id="setup-wizard" class="mb-12" aria-labelledby="heading-setup-wizard">
  <h2 id="heading-setup-wizard" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    6. Setup Wizard
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Setup Wizard runs automatically the first time you launch Jura Trace. It steps
    through a series of service checks to confirm that everything required for analysis
    is available on your device before you begin working.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What the wizard checks
  </h3>

  <div class="space-y-4 mb-6">

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 1 — Analysis Engine
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Confirms the bundled Analysis Engine (Python sidecar) is running and responding on
        port 8200. The Analysis Engine is started automatically at launch; this step
        verifies the startup completed successfully.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 2 — FFmpeg
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Checks that FFmpeg is available on your system. FFmpeg is required for video and
        audio analysis — metadata extraction, frame thumbnails, and transcription. If it
        is not found, the wizard provides installation guidance for your platform.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 3 — Speech Transcription
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Checks whether a Whisper transcription model is available. Whisper enables audio
        and video transcription, which feeds into the claim verification pipeline. This
        step prompts you to download the model if it has not yet been installed.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 4 — Local AI (Ollama)
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Checks whether Ollama is running and whether the vision model (LLaVA) and text
        model (Qwen2.5) are installed. These models are optional — all 16+ forensic
        detectors work without them. If Ollama is unavailable, the wizard confirms which
        features will be skipped.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 5 — Ready
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Summarises the results of all checks. Green items are ready. Items with warnings
        can be resolved before you continue, or skipped — Jura Trace will operate with the
        services that are available.
      </p>
    </div>

  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Re-running the wizard
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The wizard records its completion state in the application's local storage so it does
    not appear on every launch. To re-run it — for example, after installing FFmpeg or
    pulling new Ollama models — clear the application's local settings and relaunch Jura
    Trace. The wizard will run again on the next startup.
  </p>

  <!-- Callout: how to clear local storage -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Clearing local settings.</strong>
      On macOS and Linux, open the application's data directory and delete the
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">localStorage</span>
      store. On Windows, this is located in the Jura Trace application data folder under
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">%APPDATA%\com.juralabs.jura-trace\</span>.
      Note: clearing local settings resets deployment profiles and display preferences.
      Your database file and all asset records are stored separately and are not affected.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     7. Updates
     ══════════════════════════════════════════════════════════════════ -->
<section id="auto-updater" class="mb-12" aria-labelledby="heading-auto-updater">
  <h2 id="heading-auto-updater" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    7. Updates
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Jura Trace can check for and apply updates automatically. Updates are served from the
    GitHub releases page for the project. No account or registration is required.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Checking for updates
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Click <strong class="text-text-light dark:text-text-dark">Check for Updates</strong>
    in the Settings page. Jura Trace queries the GitHub releases page to determine whether
    a newer stable release is available.
  </p>

  <ul class="space-y-3 mb-6 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none">→</span>
      <span>If an update is available, a prompt appears with the new version number and the option to download and install.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none">→</span>
      <span>The update is downloaded in the background and applied when you confirm. Jura Trace will restart automatically to complete the installation.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none">→</span>
      <span>If no update is available, a brief confirmation message confirms you are on the latest version.</span>
    </li>
  </ul>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    RC and pre-release builds
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Release candidate (RC) and alpha builds are not included in the automatic update
    channel. If you are running an RC build, the updater will not offer a newer RC — it
    will only prompt when a full stable release is published. To move between pre-release
    builds, download the installer directly from the GitHub releases page.
  </p>

  <!-- Callout: update safety -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Your data is not affected by updates.</strong>
      The application database, asset records, and fingerprint history are stored
      separately from the application binary. Updating Jura Trace does not modify or
      remove any of your data.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     8. Your Plan
     ══════════════════════════════════════════════════════════════════ -->
<section id="your-plan" class="mb-12" aria-labelledby="heading-your-plan">
  <h2 id="heading-your-plan" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    8. Your Plan
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Jura Trace uses a tiered licence model. Your current tier determines which features
    are active and whether tier-specific panels appear in the results view. During the pilot
    phase, the tier can be selected manually in Settings under <strong class="text-text-light dark:text-text-dark">Your Plan</strong>.
  </p>

  <div class="space-y-4">

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Community
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Includes all core verification and protection features: C2PA signing and verification,
        perceptual fingerprinting, EXIF anomaly detection, the full set of 16+ forensic
        detectors, PDF trust reports, and ZIP case exports. Community is the default tier for
        all pilot users.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Professional
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Adds the analyst declaration header in PDF reports, raw signal scores for all
        detectors (the "Technical View" toggle in results), and batch verification of
        multiple files in a single queue. Intended for individual analysts, journalists,
        and researchers who work with sensitive or legally significant material.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Team
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Includes all Professional features. Intended for newsrooms, cultural institutions, and
        organisations deploying Jura Trace across multiple workstations, where a shared
        database location and consistent deployment configuration are needed.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Enterprise
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Includes all Team features and adds API access, advanced deployment options, and
        dedicated support. Enterprise licences are arranged directly with Juralabs and are
        scoped to the organisation's specific requirements.
      </p>
    </div>

  </div>

  <!-- Callout: tier hints are informational only -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4 mt-6">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Tier hints are informational.</strong>
      During the pilot phase, tier hints may appear on the Verify page when a feature is
      associated with a higher tier. These hints do not block any functionality — all features
      remain accessible regardless of the selected tier while the pilot is in progress.
    </p>
  </div>
</section>
