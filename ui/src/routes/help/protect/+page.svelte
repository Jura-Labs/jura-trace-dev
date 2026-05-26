<script lang="ts">
  // No reactive state needed — this is a static help page.
  // Section IDs are used for deep linking from contextual help buttons.
</script>

<!-- Signing-mode notice — Jura Trace is C2PA Validator-Conformant on the
     receive side (recordId 019d8d83-..., awarded 2026-05-06, spec 2.2).
     Generator-track conformance for the signing path is a separate
     workstream under evaluation for a future release. v1.0 signs in
     Local Signing mode (per-install Bedrock CA) only. -->
<div
  role="note"
  class="rounded-lg border border-flint/30 bg-flint/10 dark:bg-flint/5 px-4 py-3 text-sm text-text-light dark:text-quartz leading-relaxed mb-6"
>
  <strong class="font-semibold">Local Signing.</strong>
  v1.0 signs in Local Signing mode (per-install certificate authority).
  Signed files carry a valid C2PA v2.2 manifest; the signer shows as
  <code class="font-mono text-[11px]">signingCredential.untrusted</code>
  in third-party validators because our certificate isn't on the C2PA trust
  list. The manifest itself is valid; the trust scope is local to your
  installation. Conformant signing (verifiable against the C2PA trust
  list) is under evaluation for a future release. Verification is already
  C2PA Validator-Conformant.
</div>

<!-- ── Page header ────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Protecting Your Content
  </h1>
  <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
    The Protect section gives your digital assets a permanent, verifiable record of origin.
    Applying C2PA provenance lays down the first stratum of authenticity, a foundation that
    remains legible to standards-aware verifiers wherever the file goes.
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
      <a href="#what-protect-does"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. What Protect Does
      </a>
    </li>
    <li>
      <a href="#c2pa-signing"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. C2PA Provenance
      </a>
    </li>
    <li>
      <a href="#asset-management"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. Asset Management
      </a>
    </li>
    <li>
      <a href="#best-practices"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Best Practices
      </a>
    </li>
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. What Protect Does
     ══════════════════════════════════════════════════════════════════ -->
<section id="what-protect-does" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    1. What Protect Does
  </h2>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    The Protect page is your starting point for securing any digital asset. You import a
    file, and Jura Trace applies one or both of the following layers of protection:
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark font-semibold">C2PA Provenance:</strong>
        an open standard (Coalition for Content Provenance and Authenticity) that embeds a
        cryptographically signed manifest inside the file itself. The manifest records who
        created the asset, when it was signed, and a hash of the original content.
      </span>
    </li>
  </ul>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Alongside protection, the Protect page catalogues every imported asset in your local
    database. This catalogue records the file path, content type, perceptual fingerprints,
    and protection status, giving you a full registry of your collection without sending
    any data externally.
  </p>

  <!-- Callout: local-first -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">All processing is local.</strong>
      Your files never leave your device. Jura Trace does not connect to any cloud service
      during protect operations.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. C2PA Provenance
     ══════════════════════════════════════════════════════════════════ -->
<section id="c2pa-signing" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    2. C2PA Provenance
  </h2>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    C2PA provenance manifests provide a tamper-evident record of origin baked directly into
    the file. Any application that supports the C2PA standard can read and verify this
    record, including the Jura Trace Verify page and tools at
    <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">contentcredentials.org</span>.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    How to sign a file
  </h3>

  <ol class="space-y-4 mb-6">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        Open the <strong class="text-text-light dark:text-text-dark">Protect</strong> page
        from the main navigation and click <strong class="text-text-light dark:text-text-dark">Import Asset</strong>.
        Alternatively, drag and drop a file onto the import area.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        The asset appears in the asset list with its detected format and current protection
        status. Select it to open the protection panel on the right.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        Click <strong class="text-text-light dark:text-text-dark">Sign with C2PA</strong>.
        Jura Trace writes a signed manifest into the file and saves the output alongside
        the original (the source file is never overwritten).
      </span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">4</span>
      <span>
        The asset status updates to <strong class="text-text-light dark:text-text-dark">Signed</strong>,
        and the manifest details (claim generator, timestamp, and file hash) are shown in
        the protection panel.
      </span>
    </li>
  </ol>

  <div class="text-xs text-flint-dark dark:text-flint-light italic my-2" role="note">
    Visual guide: Protect page with an asset selected, the C2PA signing panel open, and a signed asset showing its manifest details.
  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3 mt-6">
    What gets embedded
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    The signed manifest records the fields below. The Sign panel shows a live
    preview of these values before you seal each file.
  </p>

  <ul class="space-y-2 mb-6 text-sm text-text-light dark:text-quartz leading-relaxed">
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Claim generator:</strong>
      identifies Jura Trace as the signing application, including the application version
      (sourced from <code class="font-mono text-xs">CARGO_PKG_VERSION</code> at build time).</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Action:</strong>
      either <code class="font-mono text-xs">c2pa.created</code> (you authored the
      content) or <code class="font-mono text-xs">c2pa.published</code> (you are
      re-distributing pre-existing content). The Sign panel asks which applies
      before sealing. When you choose <code class="font-mono text-xs">c2pa.published</code>,
      any existing manifest on the source file is preserved in the new
      manifest as a <code class="font-mono text-xs">parentOf</code> ingredient,
      so the upstream creator's signature stays in the provenance chain.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Licence + copyright:</strong>
      embedded as <code class="font-mono text-xs">stds.schema-org.CreativeWork</code> with
      <code class="font-mono text-xs">creator</code>, <code class="font-mono text-xs">copyrightNotice</code>,
      and a canonical Creative Commons URI when the selected licence has one.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">AI training and data mining:</strong>
      a <code class="font-mono text-xs">c2pa.training-mining</code> assertion declares whether
      the four canonical uses (generative training, inference, training, data mining) are
      allowed. Default is "Not allowed" for every licence except CC0, which is a public-domain
      waiver and cannot legally prohibit training.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Source type:</strong>
      conditional on the action you chose. For
      <code class="font-mono text-xs">c2pa.created</code>,
      <code class="font-mono text-xs">Iptc4xmpExt:DigitalSourceType = digitalCapture</code>
      is embedded (the IPTC vocabulary for a human-authored photograph). For
      <code class="font-mono text-xs">c2pa.published</code>, no source-type is
      claimed by the publisher because the original capture is described by the
      parent ingredient instead.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Timestamp:</strong>
      RFC 3161 trusted timestamp from a third-party TSA, so the manifest remains
      verifiable even after the signing certificate eventually expires.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span><strong class="text-text-light dark:text-text-dark">Content hash:</strong>
      a cryptographic hash of the file's pixel or media data. Any subsequent modification
      to the file will cause the hash to fail verification.</span>
    </li>
  </ul>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Supported formats
  </h3>

  <div class="overflow-x-auto mb-4">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Format</th>
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">MIME type</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">C2PA support</th>
        </tr>
      </thead>
      <tbody class="text-flint-dark dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">JPEG</td>
          <td class="py-2 pr-4 font-mono text-xs">image/jpeg</td>
          <td class="py-2 text-malachite-dark dark:text-malachite-light">✓</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">PNG</td>
          <td class="py-2 pr-4 font-mono text-xs">image/png</td>
          <td class="py-2 text-malachite-dark dark:text-malachite-light">✓</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">TIFF</td>
          <td class="py-2 pr-4 font-mono text-xs">image/tiff</td>
          <td class="py-2 text-malachite-dark dark:text-malachite-light">✓</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">WebP</td>
          <td class="py-2 pr-4 font-mono text-xs">image/webp</td>
          <td class="py-2 text-malachite-dark dark:text-malachite-light">✓</td>
        </tr>
      </tbody>
    </table>
  </div>

  <!-- Callout: tamper evidence -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Tamper evidence, not tamper prevention.</strong>
      A C2PA manifest does not prevent a file from being modified. It means that any
      modification, however small, will cause the content hash to fail when the file is
      later verified. You will see a clear warning on the Verify page if the hash does not
      match.
    </p>
  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3 mt-6">
    Metadata preservation
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    When C2PA signing completes, a confirmation panel appears on the Protect page to confirm
    that the original file's metadata was preserved in the signed copy. This matters because
    EXIF data (camera model, capture date, GPS coordinates, colour profile) forms part of
    the provenance record and must not be silently discarded during signing.
  </p>

  <ul class="space-y-3 mb-6 text-sm text-text-light dark:text-quartz leading-relaxed">
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span>The signed copy is saved alongside the original. <strong class="text-text-light dark:text-text-dark">The source file is never overwritten.</strong></span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span>All original EXIF fields are carried through into the signed output file.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite-dark dark:text-malachite-light flex-none">✓</span>
      <span>Both the original and the signed copy remain in your care, neither is sent to any external service.</span>
    </li>
  </ul>

  <!-- Callout: archival note -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Archival workflows.</strong>
      For long-term collections, keep the original unsigned file alongside the signed copy.
      The original represents the first stratum of your provenance chain. The signed copy
      provides the verifiable record of when and by whom it was deposited.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     3. Asset Management
     ══════════════════════════════════════════════════════════════════ -->
<section id="asset-management" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    3. Asset Management
  </h2>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Every file you import into the Protect page is catalogued in the local SQLite database.
    This catalogue is your permanent record of protected assets. Think of it as the
    register at the bedrock of your provenance chain.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What gets recorded
  </h3>

  <ul class="space-y-2 mb-6 text-sm text-text-light dark:text-quartz leading-relaxed">
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">File path and name:</strong>
      the location of the asset on your device at the time of import.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Content type:</strong>
      detected from the file's magic bytes (not just the extension), so the format is
      always accurate.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Perceptual fingerprints:</strong>
      three hash values are computed: aHash (average hash), dHash (difference hash), and
      pHash (perceptual hash). These fingerprints let Jura Trace identify near-duplicate
      or modified copies of an asset during verification.</span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Protection status:</strong>
      whether the asset has been signed with C2PA, and the date of the operation.</span>
    </li>
  </ul>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Filtering and browsing
  </h3>

  <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
    Use the content type filter above the asset list to show only images, video, or audio.
    The status column shows at a glance whether an asset is signed or unsigned.
  </p>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     4. Best Practices
     ══════════════════════════════════════════════════════════════════ -->
<section id="best-practices" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    4. Best Practices
  </h2>

  <ul class="space-y-4 text-sm text-text-light dark:text-quartz leading-relaxed">
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Sign before you publish.</strong>
        Apply C2PA credentials to the final version of a file before it is distributed.
        If you sign a file and then edit it, the content hash will no longer match, so you
        will need to sign again after any edits.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Back up your database.</strong>
        The local database contains your complete asset registry and signing records.
        If you lose the database, you will still have the protected files, but the
        catalogue itself will be gone. Schedule regular backups of the database file
        to a separate location.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Configure the database location for institutional deployments.</strong>
        If you are deploying Jura Trace across multiple workstations sharing a network drive,
        set a shared database path in the Settings page. This ensures all workstations
        reference the same catalogue. See
        <a href="/help/settings#database" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline">
          Settings: Database Location
        </a> for details.
      </span>
    </li>
  </ul>
</section>

<!-- ── Next steps ─────────────────────────────────────────────────── -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<section aria-label="Next steps" class="mb-4">
  <h2 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Next steps
  </h2>
  <ul class="space-y-2 text-sm">
    <li>
      <a href="/help/verify"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Verifying Content Authenticity
      </a>
      <span class="text-flint-dark dark:text-flint-light">: run forensic analysis on a file and read the trust score</span>
    </li>
    <li>
      <a href="/help/methodology"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        How Analysis Works
      </a>
      <span class="text-flint-dark dark:text-flint-light">: understand the detection methodology behind every signal</span>
    </li>
    <li>
      <a href="/help/settings"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Settings
      </a>
      <span class="text-flint-dark dark:text-flint-light">: configure database location and deployment profiles</span>
    </li>
  </ul>
</section>
