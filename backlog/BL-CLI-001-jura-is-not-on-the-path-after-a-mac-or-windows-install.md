# BL-CLI-001: `jura` is not on the PATH after a Mac or Windows install

**Status**: Open. Target v1.3.0.
**Raised**: 23 October 2026. Paul opened a new Terminal on his Mac with
1.2.0 installed and typed `jura auth status`: `zsh: command not found:
jura`.
**Severity**: Medium. The first command a new CLI user types fails, and the
walkthrough video shows `jura` working as if it were already there.

## What is wrong

- macOS: `jura` and `jura-trace-api` live inside the app bundle,
  `/Applications/Jura Trace.app/Contents/MacOS/`. Nothing puts them on the
  `PATH`. `docs/CLI_QUICKSTART.md:44-51` tells the user to `sudo ln -sf`
  both into `/usr/local/bin`.
- Windows: the installers put them in `%LOCALAPPDATA%\Jura Trace\` and do
  not add that folder to `PATH`; `CLI_QUICKSTART.md:53-60` gives a
  PowerShell snippet.
- Linux `.deb`: already on the `PATH` (`/usr/bin`). AppImage: not usable
  for CLI work (`CLI_QUICKSTART.md:36-37`).
- The in-app help (Help, Settings, section 7) says where the programs are,
  but a non-developer will not get from there to a working command.

## What would fix it

1. macOS: a Settings button, "Install the `jura` command", as editors such
   as VS Code do. It creates the two links in `/usr/local/bin`, asking for
   an administrator password through the system prompt, and offers to
   remove them again. If refused, it shows the two commands to copy.
2. Windows: the NSIS and MSI installers add `%LOCALAPPDATA%\Jura Trace` to
   the user's `PATH` (per-user, no admin), and the uninstaller removes it.
   Check how that interacts with updates.
3. `jura --version` in the help and quickstart as the check, and the help
   says plainly "open a new terminal afterwards".
4. The installer test (`installer-test.yml`) runs `jura --version` from a
   fresh shell after install, on Windows and Linux.

## What not to do

- Do not modify the user's shell profile files (`.zshrc`, `.bashrc`).
- Do not install into `/usr/local/bin` silently at first launch; it needs
  the user's consent and an admin prompt.
