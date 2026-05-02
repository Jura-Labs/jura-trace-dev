<script lang="ts">
  // Static reference page — no reactive state.
  // Created 2026-05-02 under JTV-132 after the four-agent Ollama bundling
  // evaluation. The decision was: do not bundle Ollama in v1.0 (ROOTED port
  // conflict, 8GB RAM exclusion of pilot personas, 1Mbps-bandwidth exclusion
  // of field deployment, Apple notarisation incompatibility); instead invest
  // in a frictionless optional install path. This page is the user-facing
  // documentation for that path. See `project_v1_video_audio_drop.md` agent
  // memory for the full decision record.
</script>

<article class="mx-auto max-w-3xl">

  <!-- Page heading -->
  <header class="mb-8">
    <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
      Ollama
      <span class="ml-2 align-middle text-sm font-normal text-flint-dark dark:text-flint-light">(optional)</span>
    </h1>
    <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed">
      Ollama runs two optional features locally on your machine: AI-generated image
      descriptions (LLaVA) and claim verification against a knowledge base (Qwen2.5).
      <strong class="text-text-light dark:text-text-dark">Jura Trace works fully without Ollama.</strong>
      All forensic detectors, AI deepfake detection (GBM v4 + UnivFD v9), C2PA
      provenance, and watermarking run independently.
    </p>
  </header>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Quick decision -->
  <section class="mb-10" aria-labelledby="decide-heading">
    <h2 id="decide-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Should you install it?
    </h2>
    <ul class="space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      <li><strong class="text-text-light dark:text-text-dark">Yes, if</strong> you have at least 16 GB of RAM, around 10 GB of free disk, a reliable internet connection for the one-time download (~9 GB total), and you want auto-generated image captions or claim cross-checking against a knowledge base.</li>
      <li><strong class="text-text-light dark:text-text-dark">Skip, if</strong> you are on an 8 GB machine, on a slow or metered connection, or your IT policy disallows installing software outside a managed catalogue. Core verification works fully without it.</li>
      <li><strong class="text-text-light dark:text-text-dark">Already running ROOTED?</strong> The two apps share the same Ollama instance on port 11434. You only need to download the models once.</li>
    </ul>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- macOS install -->
  <section class="mb-10" aria-labelledby="mac-heading">
    <h2 id="mac-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      macOS install
    </h2>
    <ol class="space-y-3 text-sm text-flint-dark dark:text-flint-light leading-relaxed list-decimal pl-5">
      <li>Open <strong class="text-text-light dark:text-text-dark">Settings → Service Status → Ollama</strong> and click <em>Install Ollama automatically</em>. This runs <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">brew install ollama</code> for you. If you do not have Homebrew, the manual download path is below.</li>
      <li>Wait for the install to finish (typically under a minute). Click <em>Re-check</em> in the Ollama card; the status will move from <em>Not installed</em> to <em>Installed — no models</em>.</li>
      <li>Click <em>Download</em> next to <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">llava:7b</code> and <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">qwen2.5:7b-instruct</code> in turn. Each is ~4.5 GB; download time depends on your connection. You can leave Jura Trace open during the download.</li>
      <li>When both rows show green, the Ollama card status reads <em>Ready</em>.</li>
    </ol>
    <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-4">
      Manual install: download the official package from
      <a href="https://ollama.com" target="_blank" rel="noopener noreferrer"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">ollama.com<span class="sr-only"> (opens in a new tab)</span></a>
      and run the installer. Jura Trace will detect it automatically on the next health re-check; no app restart is required.
    </p>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Windows install -->
  <section class="mb-10" aria-labelledby="win-heading">
    <h2 id="win-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Windows install
    </h2>
    <ol class="space-y-3 text-sm text-flint-dark dark:text-flint-light leading-relaxed list-decimal pl-5">
      <li>Open <strong class="text-text-light dark:text-text-dark">Settings → Service Status → Ollama</strong> and click <em>Install Ollama automatically</em>. This runs <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">winget install Ollama.Ollama</code> for you.</li>
      <li>If <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">winget</code> is not available (some Windows Server SKUs and locked-down enterprise builds), use the manual download from
        <a href="https://ollama.com" target="_blank" rel="noopener noreferrer"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">ollama.com<span class="sr-only"> (opens in a new tab)</span></a>.</li>
      <li>After install, click <em>Re-check</em> in the Ollama card and download the two models as described above.</li>
    </ol>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Linux install -->
  <section class="mb-10" aria-labelledby="linux-heading">
    <h2 id="linux-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Linux install
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Ollama provides a single-line installer. From a terminal:
    </p>
    <pre class="overflow-x-auto rounded-lg p-4 text-xs leading-relaxed mb-3"
         style="background: rgba(30,33,40,0.85); color: #EDEAE4; border: 1px solid rgba(122,119,112,0.2);"><code class="font-mono">curl -fsSL https://ollama.com/install.sh | sh</code></pre>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Then start the daemon (most Linux installs run it as a systemd service automatically; if not):
    </p>
    <pre class="overflow-x-auto rounded-lg p-4 text-xs leading-relaxed mb-3"
         style="background: rgba(30,33,40,0.85); color: #EDEAE4; border: 1px solid rgba(122,119,112,0.2);"><code class="font-mono">ollama serve &amp;</code></pre>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      Click <em>Re-check</em> in Settings, then download the models from the Ollama card.
    </p>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- ROOTED note -->
  <section class="mb-10" aria-labelledby="rooted-heading">
    <h2 id="rooted-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      If you also use ROOTED
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Jura Trace and ROOTED (Jura Labs' carbon-emissions guidance product) share the same
      Ollama instance on <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">http://localhost:11434</code>.
      Models pulled by either app are available to both. You only need to install Ollama
      once and download each model once — no duplicate disk usage.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      ROOTED uses <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">qwen2.5:7b-instruct</code>
      as its primary model, which is also one of Jura Trace's two models — so if you have
      ROOTED set up already, you only need to add <code class="font-mono text-xs px-1.5 py-0.5 rounded bg-graphite/20 dark:bg-graphite-light/20">llava:7b</code>
      from Jura Trace's Settings.
    </p>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Resource expectations -->
  <section class="mb-10" aria-labelledby="resource-heading">
    <h2 id="resource-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Disk, RAM, and download time
    </h2>
    <div class="overflow-x-auto">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Item</th>
            <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Size on disk</th>
            <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Download time at 50 Mbps</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Ollama runtime (binary)</td>
            <td class="py-2 pr-4">~90 MB</td>
            <td class="py-2">~15 s</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4"><code class="font-mono text-xs">llava:7b</code> (image descriptions)</td>
            <td class="py-2 pr-4">~4.7 GB</td>
            <td class="py-2">~13 min</td>
          </tr>
          <tr>
            <td class="py-2 pr-4"><code class="font-mono text-xs">qwen2.5:7b-instruct</code> (claim verification)</td>
            <td class="py-2 pr-4">~4.4 GB</td>
            <td class="py-2">~12 min</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-4">
      RAM usage when running: ~5–6 GB while a model is in use. Both models cannot be
      resident simultaneously on most machines — Ollama auto-evicts an idle model after
      five minutes by default, then re-loads on the next request (~5–8 s).
    </p>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Privacy + offline -->
  <section class="mb-10" aria-labelledby="privacy-heading">
    <h2 id="privacy-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Privacy and offline behaviour
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Ollama runs entirely on your machine. Once the models are downloaded, no image,
      text, or claim is sent to any external server. The models are general-purpose
      open-weight checkpoints — they do not phone home, and they do not learn from your
      data after install. The download itself is the only step that requires an internet
      connection.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      If Ollama is offline at verify time, the optional features simply do not run.
      Forensic analysis, AI deepfake detection, C2PA verification, and watermark
      extraction proceed exactly as they would without Ollama installed at all.
    </p>
  </section>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- Troubleshooting -->
  <section class="mb-10" aria-labelledby="trouble-heading">
    <h2 id="trouble-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      Troubleshooting
    </h2>
    <dl class="space-y-4 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <div>
        <dt class="font-medium text-text-light dark:text-text-dark mb-1">Status reads <em>Not installed</em> after I installed Ollama</dt>
        <dd>Click <em>Re-check</em> in the Ollama card. If still not detected, ensure the
          Ollama service is running. On macOS, open the Ollama app from <code class="font-mono text-xs">/Applications</code>.
          On Linux, run <code class="font-mono text-xs">ollama serve &amp;</code> in a terminal.
          On Windows, start <em>Ollama</em> from the Start menu.</dd>
      </div>
      <div>
        <dt class="font-medium text-text-light dark:text-text-dark mb-1">Model download stalls or fails</dt>
        <dd>Click <em>Re-check</em> to refresh the connection state, then click
          <em>Download</em> again. Ollama supports resumable downloads — partial progress
          is preserved. If the network is intermittent, downloading on a more stable
          connection is the simplest fix.</dd>
      </div>
      <div>
        <dt class="font-medium text-text-light dark:text-text-dark mb-1">Custom Ollama URL or non-default port</dt>
        <dd>The <em>AI Assistant (Ollama)</em> section near the top of Settings exposes the
          full URL and per-model identifiers. Edit them there if you run Ollama on a
          non-default port or remote host.</dd>
      </div>
    </dl>
  </section>

</article>
