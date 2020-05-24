# SAD!

Super Accelerated Diff

## What does it do?

Basically `sad` is a **Batch File Edit** tool.

It will show you a really nice diff of proposed changes *before* you commit them.

Unlike `sed`, you can double check before you fat finger your edit.

## Preview (with fzf)

Selectively replace `std` -> `joseph joestar` in the `sad` repo.

![preview1](https://github.com/ms-jpq/sad/raw/master/previews/preview1.gif)

## Preview (no fzf)

Replace all`'(\d+)'` -> `'🌈$1🌈'` in the `chromium` repo. `3GB+`

![preview2](https://github.com/ms-jpq/sad/raw/master/previews/preview2.gif)


## How to use sad?

**with fzf**

```sh
export GIT_PAGER='<highlighter-of-your-choice>'
# ^ can be done in your bash/zsh/rc file.
find "$FIND_ARGS" | sad '<pattern>' '<replacement>'
```

**without fzf**

```sh
find "$FIND_ARGS" | sad '<pattern>' '<replacement>' | highlighter-of-your-choice
```

or

```sh
find "$FIND_ARGS" | sad '<pattern>' '<replacement>' --pager=<highlighter-of-your-choice>
```

or

```sh
export GIT_PAGER='<highlighter-of-your-choice>'
find "$FIND_ARGS" | sad '<pattern>' '<replacement>'
```

---

## Requirements

Technically none of these are "required", but they make `sad` so much happier.

If you install the things below, `sad` will automatically use them. It's progressive enhancement!

### Commandline fuzzer

[**fzf**](https://github.com/junegunn/fzf)

`sad` does not come with a UI, it uses `fzf` to perform selection.

### Diff Colorizer

Any `git` compatible colourize would work. I perfer these two:

[**delta**](https://github.com/dandavison/delta)

`fd <files> | sad <pattern> <replacement> | delta`

[**diff-so-fancy**](https://github.com/so-fancy/diff-so-fancy)

`fd <files> | sad <pattern> <replacement> | diff-so-fancy | less`


## Environmental Variables

Name        | Function
------------|---------
`GIT_PAGER` | `sad` will use the same pager as `git`

## Flags

Name                                | Function
------------------------------------|---------
`-f` `--flags`                      | Regex flags, see below
`-k` `--commit`                     | No preview, write changes to file
`-0` `--read0`                      | Use `\x00` as stdin delimiter
`-e` `--exact`                      | String literal mode
`-p` `--pager`                      | Colourizing program, disable = `never`
`--fzf`                             | Additional Fzf options, disable = `never`
`-u` `--unified`                    | Same as in GNU `diff`, affects hunk size

## Regex Flags

Name | Function
-----|---------
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

## Bugs

Please file an issue if you see one `<3`
