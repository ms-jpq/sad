# SAD!

Super Accelerated Diff

## What does it do?

Basically `sad` is a **Batch File Edit** tool.

It will show you a really nice diff of proposed changes *before* you commit them.

Unlike `sed`, you can double check before you fat finger your edit.

## How to use sad?

```sh
find "$FIND_ARGS" | sad '<pattern>' '<replacement>' | highlighter-of-your-choice
```

Feed `sad` a list of files from `stdin`, a search pattern (regex by default), a replacement pattern, and you are good to go!

You can use regex capture groups. For example: `sad '"(\d+)"' '🌈$1🌈'` will replace the double quotes around integers with `🌈`.

If a replacement pattern is omitted, `sad` will assume deletion.

---

use `-k` or `--commit` to write to files


## Requirements

`sad` is designed to work with a diff colorizer. Any would work.

My recommendations are:

[diff-so-fancy](https://github.com/so-fancy/diff-so-fancy)

`fd <files> | sad <pattern> <replacement> | diff-so-fancy | less`

[delta](https://github.com/dandavison/delta)

`fd <files> | sad <pattern> <replacement> | delta`

## Previews

Replace all `"(\d+)"` -> `🌈$1🌈` in the Chromium repo.

Highlighter -- `diff-so-fancy`

![preview1](https://github.com/ms-jpq/sad/raw/master/previews/preview1.gif)

Replace all `std` -> `josephjoestar` in the Chromium repo.

Highlighter -- `delta`

![preview2](https://github.com/ms-jpq/sad/raw/master/previews/preview2.gif)

## Environmental Variables

Name        | Function
------------|----------------------------------------------------------------------------------------------------------------------------
`GIT_PAGER` | `sad` will automatically pipe it's output to the standard git pager as of v0.2. If set, no need to do `... | diff-so-fancy`

## Flags

Name                                | Function
------------------------------------|--------------------------------------------------------------------
`-i file1 file2` `--input files...` | instead of reading from `stdin`, read file names from argument list
`-k --commit`                       | instead of printing out a preview, write edits to files
`-0`                                | use `\0` instead of `\n` when reading from `stdin`
`-e` `--exact`                      | use string literal match instead of regex
`-f isx` `--flags mI`               | flags for the regex engine

## Regex Flags

Name | Function
-----|----------------------------------------------------
`i`  | case insensitive (works for `--exact` mode as well)
`I`  | case sensitive (works for `--exact` mode as well)
`m`  | multiline: `^` `$` match each line
`s`  | allow `.` match `\n`
`x`  | ignore whitespace and allow `#` comments

## GET SAD NOW!

You can download `sad` from the [github release page](https://github.com/ms-jpq/sad/releases).


## What about stdin -> stdout

If you just want to edit the shell stream, I would recommand [`sd`](https://github.com/chmln/sd), it uses the same concept, but its more for in stream edits. `sad` was inspired by my initial useage of `sd`.

```sh
command1 | sd '<pattern>' '<replacement>' | command2
```
