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
    and review licence and version information.
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
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. Ollama Configuration
     ══════════════════════════════════════════════════════════════════ -->
<section id="ollama" class="mb-12" aria-labelledby="heading-ollama">
  <h2 id="heading-ollama" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    1. Ollama Configuration
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Ollama is an optional local LLM (large language model) runtime. When running on
    your device, it extends Jura Trace with two AI-powered capabilities that are not
    available otherwise:
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Tier 3 image descriptions</strong> —
        LLaVA 7B, a vision-language model, generates a natural-language description of
        an image's contents. These descriptions are stored alongside the asset record in
        your local catalogue and can assist in identifying subject matter when searching
        or reviewing your collection.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">RAG claim verification</strong> —
        Qwen2.5 7B Instruct, a text-generation model, checks factual claims made in or
        about a piece of content against a local document store using retrieval-augmented
        generation. This is particularly useful when verifying media tied to a news event
        or institutional record.
      </span>
    </li>
  </ul>

  <!-- Callout: optional -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4 mb-6">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Ollama is entirely optional.</strong>
      All core protect and verify features — C2PA signing, watermarking, forensic analysis,
      AI-generation detection — work fully without it. Ollama adds enhanced description
      and claim-checking layers on top. If Ollama is unavailable, those layers are silently
      skipped and the rest of the analysis proceeds normally.
    </p>
  </div>

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
        Each model is approximately 4–5 GB. They are stored locally and never transmitted
        externally.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        Verify Ollama is running. In a terminal:
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
          <td class="py-2">Used for Tier 3 image descriptions. Must be a LLaVA-compatible model tag installed in your local Ollama instance.</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Text model</td>
          <td class="py-2 pr-4 font-mono text-xs">qwen2.5:7b-instruct</td>
          <td class="py-2">Used for RAG claim verification. Must be an instruction-tuned text model installed in your local Ollama instance.</td>
        </tr>
      </tbody>
    </table>
  </div>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
    Settings are saved to your browser's local storage when you click
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
    to persist the loaded values to local storage. If you do not save, the values will
    revert to the previous configuration on the next application restart.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Deleting a profile
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Click <strong class="text-text-light dark:text-text-dark">Delete</strong> next to a
    profile, then confirm the deletion. This removes the profile from local storage.
    The active settings are not affected — only the saved profile entry is removed.
  </p>

  <!-- Callout: use case example -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
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
        deepfake detection, NPR, chromatic aberration, JPEG ghost, regional analysis),
        watermark embed and extract, video and audio processing, and transcription.
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
        Start the Analysis Engine manually. In a terminal, from the
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">sidecar/</span>
        directory:
        <br />
        <code class="block mt-2 font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-3 py-2 rounded">uvicorn main:app --host 127.0.0.1 --port 8200</code>
        Then click <strong class="text-text-light dark:text-text-dark">Refresh</strong>
        in the Service Status panel.
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
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
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
    <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_archive.db</span>.
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
          <td class="py-2 font-mono text-xs break-all">~/Library/Application Support/com.juralabs.jura-trace/jura_archive.db</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold">Windows</td>
          <td class="py-2 font-mono text-xs break-all">%APPDATA%\com.juralabs.jura-trace\jura_archive.db</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold">Linux</td>
          <td class="py-2 font-mono text-xs break-all">~/.local/share/com.juralabs.jura-trace/jura_archive.db</td>
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
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_archive.db</span>
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
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_archive.db</span>
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
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Back up your database regularly.</strong>
      The
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_archive.db</span>
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
  <div class="bg-graphite rounded-lg border border-border-dark p-4 mt-6">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Local-first by design.</strong>
      Jura Trace does not collect usage data, send telemetry, or connect to any cloud
      service. Every operation — protect, verify, analyse, store — runs entirely on your
      device. Your content and your audit trail remain yours.
    </p>
  </div>
</section>
