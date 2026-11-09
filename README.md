# yt

YouTube in the macOS terminal: your home feed as a thumbnail grid, search, and a watch
page with the description, comments and related videos. Videos play in mpv.

## Requirements

- macOS 11 or later
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) and [`mpv`](https://mpv.io): `brew install yt-dlp mpv`
- A terminal with kitty graphics (Ghostty, kitty, WezTerm) or sixel for real thumbnails;
  any other terminal falls back to colored half-blocks.

## Install

```sh
cargo install --path .
```

## Signing in

yt borrows the Google session of a browser you are signed in with. On first start pick
the browser, press `o` to open Google's sign-in page there, then `⏎`. Cookies are read
from the browser on every request and never stored by yt. Press `L` later to switch or
sign out.

- Chrome, Brave, Edge: macOS asks once for Keychain access; choose "Always Allow".
- Safari: give your terminal Full Disk Access.

## Playback

Videos play at up to 1080p in the best codec this Mac decodes in hardware, as reported by
VideoToolbox: AV1 on M3 and later, VP9 otherwise. CPU use stays low and battery life
stays long.

## Files

- Settings: `~/Library/Application Support/yt/config`
- Thumbnails: `~/Library/Caches/yt`, capped at 64 MiB

## Keys

| Where       | Keys                                                                      |
| :---------- | :------------------------------------------------------------------------ |
| Home        | arrows/`hjkl` move, `⏎` play, `tab` next feed, `/` search, `r` refresh     |
| Watch page  | `↑↓` move, `⏎` expand description/replies, `p` play again, `esc` back     |
| Up next     | `→`/`tab` go there, `↑↓` choose, `⏎` play, `←` back to the details        |
| Search bar  | `⏎` search, `esc` cancel, `^U` clear, `^W` delete word                    |
| Anywhere    | `L` account, `q` quit                                                     |
