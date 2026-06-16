Jura Trace - Practice Images
============================

Four files that show what each kind of verification result looks like, so you
can recognise them before testing your own content. Open the Verify tab and
drop each file in. Scores are approximate and depend on the model versions in
your build.

1. authentic.jpg
   A real photograph (Google Pixel 8, February 2025). Location and serial
   metadata were removed; the camera identification was kept.
   Expected result: High Trust, around 84 per cent. Camera metadata present,
   no manipulation or AI signals.

2. ai-generated-openai.png
   An AI-generated image made with OpenAI's image model. It carries valid C2PA
   Content Credentials issued by the OpenAI Media Service API (8 June 2026)
   that declare it as trained algorithmic media.
   Expected result: Low Trust, around 15 per cent. The AI detector reads it as
   synthetic, and the Content Credentials confirm it was AI-generated.
   The lesson: a valid Content Credential proves where an image came from, not
   that it is a real photograph. Here the credential is valid and it tells you
   the image is AI-generated.

3. resaved-uncertain.jpg
   The authentic photo above, downscaled and re-saved the way a messaging app
   or social platform would, with its metadata stripped.
   Expected result: Uncertain, around 55 per cent. Re-saving weakened the
   signals the checks rely on.
   The lesson: "Uncertain" does not mean fake. It often means the file was
   re-saved, cropped, screenshotted, or forwarded through a messaging app.
   Where you can, verify the original file.

4. jura-trace-signed.jpg
   The authentic photo, signed with Jura Trace in Local Signing mode (the
   Protect tab).
   Expected result: High Trust, around 94 per cent, with Jura Trace Content
   Credentials shown. Other tools may mark the signer "untrusted" because the
   certificate is not on a shared trust list. That is expected in Local
   Signing mode and does not mean the signature failed.

Provenance and licence
----------------------
The AI image was generated via OpenAI; the output is owned by the generator
under OpenAI's terms and is used here with permission. The authentic photo is
shared under CC0-1.0. No identifiable private individual is the subject of
these images (the AI image depicts synthetic people).
