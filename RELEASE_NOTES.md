## Good news

v0.4.33

- Match filenames in fzf without matching trailing hunk metadata.

- Dim the hunk metadata in fzf.

- Restore default Ctrl-C handling so blocking work terminates immediately.

- Exit immediately with status 130 when fzf aborts, including while stdin remains open.

---

v0.4.32

- It looks like in the newer versions of `fzf` the command that was used to "execute and exit" no longer works.

  - A quick fix is to simply migrate to `fzf`'s new `become` command, which almost does the same thing.

  - This is a breaking change for older versions of `fzf` however, and users using `fzf` shipped with ubuntu 22 and below should stick with old version for now

v0.4.31

- Bump version purely for CI

v0.4.30

- Fix display issue with new fzf ( no longer use `\n` as delimiter )

v0.4.29

- Revert multi-select preview, since it conflicts with incremental preview :(

- Add PowerShell flag '\r' flag under windows

v0.4.28

- Give up on trying to solve windows edge cases, instead of not working at all, it now works for utf-8 only

v0.4.26

- Reduce allocations

- Preview multi-selects

- Fewer dependencies

- More robust signal handling

**Released by CI**
