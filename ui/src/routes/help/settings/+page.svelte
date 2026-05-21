<script lang="ts">
  // No reactive state needed — this is a static help page.
  // Section IDs are used for deep linking from contextual help buttons.
</script>

<!-- ── Page header ────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Application Settings
  </h1>
  <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
    The Settings page lets you manage named deployment profiles, check service health,
    choose where your local database is stored, and review licence and version information.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- ── Table of contents ───────────────────────────────────────────── -->
<nav aria-label="Page contents" class="mb-10">
  <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-3">
    On this page
  </p>
  <ol class="space-y-1 text-sm">
    <li>
      <a href="#ollama"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. Ollama Configuration
      </a>
    </li>
    <li>
      <a href="#deployment-profiles"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. Deployment Profiles
      </a>
    </li>
    <li>
      <a href="#service-status"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. Service Status
      </a>
    </li>
    <li>
      <a href="#database"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Database Location
      </a>
    </li>
    <li>
      <a href="#about"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        5. About
      </a>
    </li>
    <li>
      <a href="#setup-wizard"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        6. Setup Wizard
      </a>
    </li>
    <li>
      <a href="#auto-updater"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        7. Updates
      </a>
    </li>
    <li>
      <a href="#your-plan"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Ollama is a free tool that runs AI models locally. Jura Trace v1.0 ships with
    no Ollama-dependent features enabled. The AI Assistant section is reserved for
    v1.0.1, where a single multimodal+text model will provide image descriptions and
    claim verification. Until then, this section is hidden in Settings and no Ollama
    install is required.
  </p>

  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">All v1.0 features run without Ollama</p>
    <ul class="space-y-1 text-sm text-flint-dark dark:text-flint-light">
      <li class="flex gap-2"><span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span> 10 automatic forensic detectors plus 3 on-demand investigation tools</li>
      <li class="flex gap-2"><span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span> C2PA Validator-Conformant verification</li>
      <li class="flex gap-2"><span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span> EXIF anomaly analysis with injection-detection and XMP AI-provenance sub-checks</li>
      <li class="flex gap-2"><span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span> AI deepfake detection (GBM v4 + UnivFD v10onnx ensemble)</li>
      <li class="flex gap-2"><span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span> Local Signing with per-install certificate</li>
    </ul>
  </div>

</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. Deployment Profiles
     ══════════════════════════════════════════════════════════════════ -->
<section id="deployment-profiles" class="mb-12" aria-labelledby="heading-profiles">
  <h2 id="heading-profiles" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    2. Deployment Profiles
  </h2>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Deployment profiles let you save your current Ollama configuration under a name and
    switch between saved configurations quickly. This is useful when you use Jura Trace
    in different contexts (for example, when working in an office with access to a
    shared institutional Ollama server versus working offline in the field).
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Saving a profile
  </h3>

  <ol class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Configure the Ollama URL and model names to match the environment you want to save.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Click <strong class="text-text-light dark:text-text-dark">Save as profile</strong>
        in the Deployment Profiles section. Enter a name (for example,
        "Office" or "Field") and confirm.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Click <strong class="text-text-light dark:text-text-dark">Load</strong> next to the
    profile you want to apply. The Ollama URL and model fields update immediately to
    reflect the saved values. A confirmation message appears briefly to confirm the
    profile has been loaded.
  </p>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    After loading a profile, click
    <strong class="text-text-light dark:text-text-dark">Save settings</strong>
    to persist the loaded values to the application's local settings. If you do not save, the values will
    revert to the previous configuration on the next application restart.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Deleting a profile
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
    Click <strong class="text-text-light dark:text-text-dark">Delete</strong> next to a
    profile, then confirm the deletion. This removes the profile from the application's local settings.
    The active settings are not affected; only the saved profile entry is removed.
  </p>

  <!-- Callout: use case example -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    The Service Status panel shows the current availability of the two optional services
    that extend Jura Trace's analysis capabilities. Both services are optional; the core
    protect and verify features work without them.
  </p>

  <div class="space-y-5 mb-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Analysis Services
      </h3>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
        This indicator shows the status of the Analysis Engine, which runs locally on
        the 127.0.0.1 loopback interface. The Analysis Engine provides forensic image analysis (ELA, noise, copy-move,
        deepfake detection, JPEG ghost, regional analysis),
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
          <tbody class="text-flint-dark dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-malachite-dark dark:text-malachite-light">Online</td>
              <td class="py-2">The Analysis Engine is running and responding. Forensic analysis is available.</td>
            </tr>
            <tr>
              <td class="py-2 pr-4 font-semibold text-cinnabar-dark dark:text-cinnabar-light">Offline</td>
              <td class="py-2">The Analysis Engine is not running or is not reachable. Forensic analysis will be skipped during verification. C2PA checking and EXIF analysis still work.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Ollama
      </h3>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
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
          <tbody class="text-flint-dark dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-malachite-dark dark:text-malachite-light">Online</td>
              <td class="py-2">Ollama is running and at least one model is available. AI-assisted features are reserved for v1.0.1. All current v1.0 analysis continues normally.</td>
            </tr>
            <tr>
              <td class="py-2 pr-4 font-semibold text-cinnabar-dark dark:text-cinnabar-light">Unavailable</td>
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
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        The Analysis Engine starts automatically when you launch Jura Trace. If it shows as
        <strong class="text-text-light dark:text-text-dark">Offline</strong>, try restarting
        the application. The Engine binds to a free local port chosen at startup, so port
        conflicts are unlikely. Once the service is running, click
        <strong class="text-text-light dark:text-text-dark">Refresh</strong> in the Service
        Status panel to confirm the connection.
      </p>
    </div>

    <div>
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Ollama unavailable
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Jura Trace stores all asset records, fingerprints, verification history, and audit
    logs in a single SQLite database file named
    <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>.
    The current file path is shown at the top of the Database Location section.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Default locations
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
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
      <tbody class="text-flint-dark dark:text-flint-light">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
    Jura Trace resolves the database path in the following order of precedence. The first
    valid path found is used:
  </p>

  <ol class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Environment variable:</strong>
        if the
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">JURA_DB_PATH</span>
        environment variable is set to a valid file path, that path is used. Useful for
        scripted or institutional deployments.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Config file:</strong>
        if a
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">config.json</span>
        file in the application data directory contains a
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">db_path</span>
        key pointing to a valid file, that path is used. This file is written
        automatically when you change the database location via the Settings page.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Default:</strong>
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>
        in the platform application data directory, as shown in the table above.
      </span>
    </li>
  </ol>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Moving the database
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    You may want to move the database to a different location (for example, to a shared
    network drive accessible to colleagues, or to a directory governed by your
    institution's backup policy).
  </p>

  <ol class="space-y-4 mb-6">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Click <strong class="text-text-light dark:text-text-dark">Change location</strong>
        in the Database Location section.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Select the destination folder in the file picker. Jura Trace will use the name
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">jura_trace.db</span>
        in the chosen folder.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        Jura Trace copies the existing database to the new location, verifies the copy is
        intact, then updates
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">config.json</span>
        to point to the new path. The original file remains in place until the move is
        confirmed.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">4</span>
      <span>
        Restart the application for the new path to take full effect. Jura Trace will
        open the database at the new location on startup.
      </span>
    </li>
  </ol>

  <!-- Callout: backup -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
    The About section displays the installed application version and licensing information.
  </p>

  <dl class="space-y-4">
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Application</dt>
      <dd class="mt-1 text-sm text-text-light dark:text-quartz leading-relaxed">Jura Trace</dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Developer</dt>
      <dd class="mt-1 text-sm text-text-light dark:text-quartz leading-relaxed">
        Juralabs Community Interest Company (UK),
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">juralabs.org</span>
      </dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Licence</dt>
      <dd class="mt-1 text-sm text-text-light dark:text-quartz leading-relaxed">
        AGPL-3.0-or-later. You may use, study, modify, and redistribute this software
        for any purpose (including commercial use), provided you comply with the AGPL's
        terms (notably its source-disclosure requirement for hosted/network deployments).
        A separate commercial licence is available for use cases that cannot operate
        under the AGPL. See the
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">LICENSE</span>
        and
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">COMMERCIAL.md</span>
        files for full terms.
      </dd>
    </div>
    <div>
      <dt class="text-sm font-semibold text-text-light dark:text-text-dark">Version</dt>
      <dd class="mt-1 text-sm text-text-light dark:text-quartz leading-relaxed">
        The current version number is shown in the Settings page. To report a bug or
        request a feature, visit the project repository or contact
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">juralabs.org</span>.
      </dd>
    </div>
  </dl>

  <!-- Callout: local-first reminder -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4 mt-6">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Local-first by design.</strong>
      Jura Trace does not collect usage data, send telemetry, or connect to any cloud
      service. Every operation (protect, verify, analyse, store) runs entirely on your
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
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
        Step 1: Analysis Engine
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Confirms the bundled Analysis Engine (Python sidecar) is running and responding on
        the 127.0.0.1 loopback interface. The Analysis Engine is started automatically at
        launch; this step verifies the startup completed successfully.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 2: FFmpeg
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Checks that FFmpeg is available on your system. FFmpeg is required for video and
        audio analysis: metadata extraction, frame thumbnails, and transcription. If it
        is not found, the wizard provides installation guidance for your platform.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 3: Speech Transcription
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Checks whether a Whisper transcription model is available. Whisper enables audio
        and video transcription, which feeds into the claim verification pipeline. This
        step prompts you to download the model if it has not yet been installed.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 4: Local AI (Ollama)
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Checks whether Ollama is running and whether the vision model (LLaVA) and text
        model (Qwen2.5) are installed. These models are optional; all 10 automatic
        forensic detectors work without them. If Ollama is unavailable, the wizard
        confirms which features will be skipped.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Step 5: Ready
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Summarises the results of all checks. Green items are ready. Items with warnings
        can be resolved before you continue, or skipped. Jura Trace will operate with the
        services that are available.
      </p>
    </div>

  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Re-running the wizard
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    The wizard records its completion state in the application's local storage so it does
    not appear on every launch. To re-run it (for example, after installing FFmpeg or
    pulling new Ollama models), clear the application's local settings and relaunch Jura
    Trace. The wizard will run again on the next startup.
  </p>

  <!-- Callout: how to clear local storage -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
    Jura Trace can check for and apply updates automatically. Updates are served from the
    GitHub releases page for the project. No account or registration is required.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Checking for updates
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Click <strong class="text-text-light dark:text-text-dark">Check for Updates</strong>
    in the Settings page. Jura Trace queries the GitHub releases page to determine whether
    a newer stable release is available.
  </p>

  <ul class="space-y-3 mb-6 text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Release candidate (RC) and alpha builds are not included in the automatic update
    channel. If you are running an RC build, the updater will not offer a newer RC; it
    will only prompt when a full stable release is published. To move between pre-release
    builds, download the installer directly from the GitHub releases page.
  </p>

  <!-- Callout: update safety -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
    Jura Trace v1.0 ships as a single Community release, free for everyone under the
    AGPL-3.0-or-later licence. Bespoke Custom Engineering is available for institutions
    that need integration, training, or compliance support.
  </p>

  <div class="space-y-4">

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Community (v1.0)
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        The full Jura Trace application: C2PA Validator-Conformant verification, Local
        Signing, ten automatic forensic detectors, three on-demand investigation tools,
        perceptual fingerprinting, EXIF anomaly detection with injection-detection and
        XMP AI-provenance sub-checks, Watched Locations folder monitoring, PDF trust
        reports, ZIP case exports. Free for everyone under AGPL-3.0-or-later. Permanent:
        the CIC's asset-locked status guarantees a free Community release in perpetuity.
      </p>
    </div>

    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
      <p class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
        Custom Engineering
      </p>
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        Bespoke engagements for institutions needing integration, training, custom-RAG
        knowledge bases, MDM packaging, or compliance documentation packs. Also covers
        embedding Jura Trace in your own product without AGPL contagion (a commercial
        licence is included in the engagement). Quoted per engagement, typically from
        £5,000 / 5 days. Email
        <a
          href="mailto:consultancy@juralabs.org"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >consultancy@juralabs.org</a>
        for a quote.
      </p>
    </div>

  </div>
</section>
