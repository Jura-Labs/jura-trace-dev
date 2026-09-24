# BL-CLAIM-004: Standard network mode does not cover the signing timestamp

**Status**: Open. Found 13 September 2026 while writing the A3 correction for
v1.2.0, which is the rewrite of the README sentence BL-CLAIM-001 is about.
**Raised**: 13 September 2026
**Severity**: High. It is the one outbound call that carries data derived from
user content, and it fires for the users who explicitly asked for no outbound
calls.

## What is wrong

`src-tauri/src/network_mode.rs:14` states the invariant:

> All network-touching code paths call [`is_enhanced`] before making any
> network call.

`src-tauri/src/c2pa.rs` contains no reference to `is_enhanced`, `network_mode`
or `NetworkMode`. I grepped the file. `TSA_URL`
(`http://timestamp.digicert.com`, `c2pa.rs:323`) is passed unconditionally to
`c2pa::create_signer::from_keys` at `c2pa.rs:608`.

There are six `is_enhanced` call sites outside the module itself, all in
`lib.rs` (`:1056`, `:1087`, `:1120`, `:1175`) and `monitor_scheduler.rs`
(`:177`). None is on the signing path.

So a user who has gone into Settings and chosen Standard, which
`network_mode.rs:39` describes as "explicit user opt-in for fully local /
air-gapped operation", still makes an outbound HTTP request to a DigiCert
server every time they sign a file.

## How it showed up

Writing the replacement for `README.md:7` under v1.2.0 item A3. The natural
sentence to write was "Standard network mode makes none of these calls",
because that is what the module documentation says. Checking it before
publishing it is what found this. The README as merged says instead that the
timestamp request "is not covered by that switch today", which is true and is
not a good sentence to have to write.

BL-CLAIM-001's table lists this call as firing "Every time the user signs a
file", with no mode caveat, so the fact was visible on 3 September. What was
not noticed is that it contradicts the Standard-mode promise.

## Why it matters

Three reasons, in order.

**The data is the sensitive kind.** The legal review recorded in BL-CLAIM-001
on 3 September singled this call out: it sends a hash *derived from the user's
content*. The other four calls leak IP address and activity timing. This one
is the reason "no data is uploaded" was assessed as literally false rather
than merely imprecise.

**The population is the wrong one to get this wrong for.** `network_mode.rs`
argues its own defaults from exactly this: "the risk to a Standard user under
surveillance is unrecoverable once sent" (`:79-82`). Standard is not the
default. Enhanced is, since the Validator conformance work. So every Standard
user is a user who went looking for the setting and chose it deliberately, and
the reason a person does that is the reason this matters.

**It is a control that does not cover what it says it covers.** That is the
BL-SILENT-001 class: the switch reports a state it does not fully deliver. The
user sees Standard, reads air-gapped, and signs.

## What is already right

This is disclosed, and that is real mitigation rather than an excuse. The
protect page shows a pre-seal panel listing the timestamp authority by URL
(`ui/src/routes/protect/+page.svelte:1821` and `:3377`) and states that what
goes out is "Jura Trace + timestamp + content hash" (`:1824`, `:3380`), fed by
`get_signing_disclosure` at `lib.rs:2888-2915`. A user who reads the panel
before sealing is told. The defect is that their earlier, explicit, global
choice is not honoured, not that they are deceived at the moment of signing.

## What would fix it

The decision is not which line of code to add. It is what signing should do
for a Standard-mode user, and there are only three honest answers.

1. **Refuse to sign, and say why.** Cleanest reading of "air-gapped". Cruel if
   the user just wants to seal a file offline and does not care about trusted
   time.
2. **Sign without a timestamp, and mark the manifest as untimestamped.** The
   c2pa API takes `Option<String>` for the TSA, so passing `None` is
   mechanically trivial. The cost is a weaker signature: no proof of when. The
   UI must then say the seal has no trusted time, and the verify side must
   render that state, which is the real work.
3. **Ask, once, at the first signing attempt in Standard mode**, and remember
   the answer. Probably the right one, because the trade-off is genuinely the
   user's and neither default is obviously correct.

Whichever is chosen, `network_mode.rs:14` is corrected in the same change,
because its invariant is currently false and the next person to read it will
believe it.

Check the verify side too. If a manifest with no timestamp validates
differently, or renders no differently, that needs settling before option 2
ships.

## What not to do

- Do not silently pass `None` for the TSA in Standard mode and leave the UI
  unchanged. That trades a disclosed outbound call for an undisclosed downgrade
  in signature strength, which is a worse defect and a harder one to find.
- Do not describe this as a bug in c2pa-rs. The library takes an `Option`. The
  product passes `Some` unconditionally.
- Do not fix the documentation instead of the code. Rewording
  `network_mode.rs:14` to describe what the code does would make the invariant
  true and the product no safer. The invariant is the thing worth keeping.
- Do not fold this into A2, the Swagger pinning. Different call, different
  population, different decision.

## Decided and implemented, 24 September 2026

Paul chose option 3, ask once. Branch `w39-25-claim-004-ask-once`.

- `network_mode::signing_timestamp` is the policy. Enhanced mode timestamps.
  Standard mode follows the answer remembered in `network_mode.json`
  (`standard_mode_timestamp`), and with no answer returns `Ask`, where
  signing is refused. That refusal is in `sign_asset` and in the REST API's
  sign route (409 `TimestampChoiceRequired`), not only in the UI, so no
  caller can skip the question. An unreadable config counts as not asked.
  `set_network_mode` keeps the answer when the mode changes.
- `c2pa::sign_file` and `sign_file_with_active_mode` take the TSA URL as a
  parameter. `None` signs without a timestamp. A new test proves such a seal
  validates on read-back and carries no signing time.
- The Protect page asks before a single or batch signing when the answer is
  missing. Settings, Network Access can change the answer or reset it to ask
  again. The pre-seal disclosure shows no timestamp authority when the answer
  is no.
- The verify page and the PDF report now say "No trusted timestamp" instead
  of omitting the date.
- `network_mode.rs`'s header no longer claims Standard mode makes no outbound
  requests of any kind. It lists the three calls a person asks for.

Resolved: 2026-09-24 b7cc1e54 — PR #112 merged. Option 3, ask once: Standard-mode signing asks before requesting a trusted timestamp, remembers the answer, and refuses to sign until answered.
