<script lang="ts">
  // No reactive state — static help page.
  // Section IDs are used for deep linking from Settings.
</script>

<!--
  Help — Bedrock and Conformant Signing
  =====================================
  Explains Jura Trace's two C2PA signing modes, why Bedrock is the
  default, when to switch to Conformant, and why external validators
  mark Bedrock-signed files as "untrusted".

  Sanctuary theme. British spelling. No emojis. WCAG 2.2 AA.
-->

<article aria-labelledby="bedrock-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <h1
      id="bedrock-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Local and Conformant Signing
    </h1>
    <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
      Jura Trace signs C2PA provenance manifests in one of two modes. The default
      (<strong>Local Signing</strong>) works entirely offline, on your device,
      with no account, no data sent to Juralabs, and no dependency on any external service.
      The optional <strong>Conformant Signing</strong> mode accepts a certificate
      from a C2PA-approved authority for institutions that need cross-tool
      interoperability with Adobe, BBC, and other conformant validators.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Table of contents ────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-3">
      On this page
    </p>
    <ol class="space-y-1 text-sm">
      <li><a href="#bedrock" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">What Local Signing does</a></li>
      <li><a href="#conformant" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">When to use Conformant Signing</a></li>
      <li><a href="#untrusted" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">The &ldquo;untrusted&rdquo; warning explained</a></li>
      <li><a href="#obtaining-cert" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Obtaining a Conformant certificate</a></li>
      <li><a href="#switching-modes" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Switching modes</a></li>
      <li><a href="#which-should-i-use" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Which mode should I use?</a></li>
    </ol>
  </nav>

  <!-- ── Bedrock overview ─────────────────────────────────────────────── -->
  <section aria-labelledby="bedrock-overview-heading" id="bedrock" class="mb-12">
    <h2
      id="bedrock-overview-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      What Local Signing does
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      The first time you protect a file with Jura Trace, the application generates a
      small certificate authority on your device using industry-standard ECDSA
      P-256 cryptography. That authority issues a signing certificate tied to this
      specific install of Jura Trace. Every file you protect from that point
      onwards is signed with the local certificate.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Local Signing was designed around four institutional concerns that
      cloud-based provenance services cannot answer:
    </p>

    <div class="space-y-4 mb-6">

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Offline by default</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          No network connection is required at any stage: not to generate the
          signing certificate, not to sign a file, not to verify a signature. A
          laptop in an air-gapped archive room, a camera in a conflict zone, or a
          processing station on a research vessel can protect and verify content
          with identical behaviour to a fully connected desktop.
        </p>
      </div>

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">No shared signing authority</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          Every install of Jura Trace generates its own certificate authority.
          There is no single upstream entity that can be compelled to revoke your
          signatures, no central service that can go offline and break your
          workflow, and no pooled identity that ties your institution's work to
          any other user of the software.
        </p>
      </div>

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">No telemetry, no enrolment</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          Local Signing does not transmit the certificate, the signing events, or any
          metadata about what you are protecting. The cryptographic keys live on
          your device only. There is no account to create, no support portal to
          register with, and nothing to reach out to Juralabs about unless you
          choose to.
        </p>
      </div>

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Technically sound C2PA manifests</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          Locally signed files carry a fully valid C2PA manifest that matches the
          structural requirements of the C2PA Technical Specification version 2.2.
          The manifest can be read and parsed by any C2PA-compatible tool worldwide,
          including Adobe Content Authenticity, the public
          <span class="font-mono">contentcredentials.org/verify</span> inspector,
          and any downstream validator implementing the specification.
        </p>
      </div>

    </div>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Conformant overview ──────────────────────────────────────────── -->
  <section aria-labelledby="conformant-heading" id="conformant" class="mb-12">
    <h2
      id="conformant-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      When to use Conformant Signing
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Conformant Signing is the optional second mode. Instead of using the
      per-install local certificate authority, Jura Trace signs files with a
      certificate issued to your institution by a C2PA-approved certification
      authority on the official trust list. Manifests produced this way validate
      cleanly, with no warnings, in every C2PA validator worldwide.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Conformant is the right choice when any of the following apply:
    </p>

    <ul class="list-disc pl-5 space-y-2 text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      <li>Your institution has a formal procurement process that requires a
        certificate chain traceable to a named, recognised certificate authority.</li>
      <li>Your downstream workflow includes Adobe Creative Cloud, BBC Verify
        tooling, or any enterprise system that will only accept manifests signed
        by a trust-list certificate.</li>
      <li>You want the files you sign to carry a verifiable institutional
        identity (museum name, press masthead, legal firm) in the cross-tool
        display, not only inside Jura Trace.</li>
      <li>Your compliance or audit framework requires third-party attestation of
        the signing identity.</li>
    </ul>

    <div class="bg-lapis/5 dark:bg-lapis/10 border border-lapis/20 dark:border-lapis/30 rounded-lg p-5 mb-4">
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        <strong>Local and Conformant produce structurally identical manifests.</strong>
        The only difference is the root of trust: whether the signing certificate
        chains back to a public trust list or to the per-install local authority.
        You can switch modes at any time in Settings. Existing signed files retain
        whichever signature they were signed with; new files use the active mode.
      </p>
    </div>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── The "untrusted" warning ──────────────────────────────────────── -->
  <section aria-labelledby="untrusted-heading" id="untrusted" class="mb-12">
    <h2
      id="untrusted-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      The &ldquo;untrusted&rdquo; warning explained
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      When you open a locally signed file in an external C2PA validator (Adobe
      Inspect, contentcredentials.org/verify, or a desktop tool from another
      vendor), you will see a warning that the signing credential is
      &ldquo;untrusted&rdquo;. This warning is expected, and understanding why it
      appears is important for interpreting it correctly.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      External validators maintain a curated list of certificate authorities whose
      certificates they will accept without a warning. This list is called the
      <strong>C2PA trust list</strong>. The per-install local certificate authority is not
      on that list, and cannot be, because the authority was generated on your
      device and is unique to your install. There is no mechanism by which a
      public trust list could include millions of per-install local authorities.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      <strong>This does not mean the signature is broken or the manifest is
      invalid.</strong> The cryptographic signature verifies correctly. The
      assertions inside the manifest are intact. The binding between the asset
      and the manifest is sound. The validator is telling you, precisely, that
      it cannot look up the issuing authority in its configured trust list,
      which is a truthful statement about an identity decision, not a verdict on
      the integrity of the file.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      For use cases where this warning is acceptable (institutional archives,
      internal workflows, Jura Trace-to-Jura Trace verification, evidence chains
      whose audit trail is separate from the cryptographic signature), Local Signing is
      the right choice. For use cases where the warning is not acceptable
      (cross-vendor interoperability, public-facing provenance display, enterprise
      procurement gates), switch to Conformant Signing.
    </p>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Obtaining a certificate ──────────────────────────────────────── -->
  <section aria-labelledby="obtaining-cert-heading" id="obtaining-cert" class="mb-12">
    <h2
      id="obtaining-cert-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Obtaining a Conformant certificate
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Conformant Signing requires a certificate issued by a certificate authority
      on the C2PA trust list. Jura Labs does not issue these certificates; your
      institution applies for one directly from an approved authority. Typical
      steps:
    </p>

    <ol class="list-decimal pl-5 space-y-2 text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      <li>Confirm the current trust list at the C2PA Conformance Explorer at
        <span class="font-mono">c2pa-org.github.io/conformance-explorer</span>.
        The list of approved authorities evolves as the programme matures.</li>
      <li>Contact an authority offering content-signing certificates for the C2PA
        programme. Pricing varies by authority and assurance level; typical
        annual ranges run from a few hundred to a few thousand pounds.</li>
      <li>Complete the identity verification required by the authority. This
        usually involves confirming the institution's legal registration,
        authorised signatories, and use of the certificate.</li>
      <li>The authority will issue a certificate file (commonly a PEM-formatted
        chain) and a corresponding private key. Store both securely: the private
        key should be protected with the same care as any other cryptographic
        secret.</li>
      <li>Import both files into Jura Trace via the Signing Mode section of the
        Settings page. Jura Trace validates the certificate against the C2PA
        certificate profile and verifies that the key matches the certificate
        before accepting it.</li>
    </ol>

    <div class="bg-malachite/5 dark:bg-malachite/10 border border-malachite/30 dark:border-malachite/40 rounded-lg p-5 mb-4">
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        <strong>Jura Trace holds C2PA Validator-Conformant status</strong>
        (recordId <span class="font-mono text-xs">019d8d83-ed1c-787c-920c-8fad67b55cbe</span>,
        spec 2.2), awarded 6 May 2026 and listed on the C2PA Conforming Products
        List from 31 May 2026. Generator-track conformance, which would cover the
        signing side of the dual-mode architecture under Conformant, is a separate
        Generator-track conformance for the signing path is under evaluation for a future release. Verification is already C2PA Validator-Conformant. Neither award is required for
        Local Signing, which operates entirely outside the conformance programme's
        trust model by design.
      </p>
    </div>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Switching modes ──────────────────────────────────────────────── -->
  <section aria-labelledby="switching-heading" id="switching-modes" class="mb-12">
    <h2
      id="switching-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Switching modes
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      The active signing mode is controlled from the <strong>Signing Mode</strong>
      section of the Settings page. Switching modes takes effect immediately on
      the next file you protect, with no restart, no re-onboarding, no migration
      step.
    </p>

    <ul class="list-disc pl-5 space-y-2 text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      <li><strong>Local → Conformant:</strong> import a valid certificate and
        key via the Import Certificate button. After successful validation, Jura
        Trace offers to switch the active mode in a single click.</li>
      <li><strong>Conformant → Local:</strong> click the Switch to Local Signing
        button in the Local Signing card. Your imported Conformant certificate is
        retained, so switching back to Conformant later does not require
        re-importing.</li>
      <li><strong>Removing the Conformant certificate:</strong> click Remove in
        the Conformant card. The certificate and key files are deleted from the
        device and the active mode reverts to Local Signing.</li>
    </ul>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Files signed in one mode retain their signature if you later switch modes.
      Changing the active mode only affects files you protect from that point
      onwards. If you want to re-sign an existing file under a different mode,
      use the Protect workflow on the original file again. Jura Trace will
      produce a new signed copy under the currently active mode.
    </p>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Which should I use ───────────────────────────────────────────── -->
  <section aria-labelledby="which-heading" id="which-should-i-use" class="mb-12">
    <h2
      id="which-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Which mode should I use?
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      The right choice depends on what happens to your content after it leaves
      Jura Trace and who needs to verify it.
    </p>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-3">Local Signing fits when</h3>
        <ul class="list-disc pl-4 space-y-1.5 text-sm text-text-light dark:text-quartz leading-relaxed">
          <li>Your work is catalogued and verified inside your institution</li>
          <li>Your files are stored in an air-gapped archive or processed offline</li>
          <li>Your workflow is Jura Trace-to-Jura Trace or uses a self-hosted validator</li>
          <li>You document in hostile or low-connectivity environments</li>
          <li>You prefer sovereign cryptographic identity with no external dependency</li>
          <li>Your evidence chain relies on a separate institutional audit trail alongside the manifest</li>
        </ul>
      </div>

      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-3">Conformant fits when</h3>
        <ul class="list-disc pl-4 space-y-1.5 text-sm text-text-light dark:text-quartz leading-relaxed">
          <li>Your files will be verified by external parties using Adobe or other vendor tools</li>
          <li>Your downstream workflow includes enterprise procurement or compliance gates</li>
          <li>Your institution has been issued a C2PA trust-list certificate</li>
          <li>You want the signing identity displayed consistently across vendors</li>
          <li>You are publishing to public audiences expecting a &ldquo;verified&rdquo; badge in third-party tools</li>
          <li>You need third-party attestation of the signing identity for audit purposes</li>
        </ul>
      </div>

    </div>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      If you are unsure, start with Local Signing. It is the default for a reason,
      and switching to Conformant later is a single-step import. You do not need
      to re-protect existing files when you switch modes; the existing signed
      files remain valid under their original signing mode.
    </p>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Related help ─────────────────────────────────────────────────── -->
  <section aria-labelledby="related-heading" class="mb-12">
    <h2
      id="related-heading"
      class="font-heading text-xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Related help
    </h2>

    <ul class="list-disc pl-5 space-y-2 text-sm text-text-light dark:text-quartz leading-relaxed">
      <li><a href="/help/protect" class="text-lapis dark:text-lapis-light underline hover:no-underline">Protect workflow</a>: the full Protect page covering C2PA signing, watermarking, and fingerprinting.</li>
      <li><a href="/help/settings" class="text-lapis dark:text-lapis-light underline hover:no-underline">Settings</a>: where the Signing Mode toggle lives, alongside other Jura Trace configuration.</li>
      <li><a href="/help/continuity" class="text-lapis dark:text-lapis-light underline hover:no-underline">Continuity Promise</a>: what happens to your signed content if Jura Labs CIC ever ceases operations.</li>
      <li><a href="/help/glossary" class="text-lapis dark:text-lapis-light underline hover:no-underline">Glossary</a>: definitions of C2PA, trust list, certificate authority, and other terms used on this page.</li>
    </ul>
  </section>

</article>
