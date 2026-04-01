# wlr

`wlr` is a small streaming filter for ANSI-colored command output. It keeps lines whose visible text matches one or more target colors and can optionally include context lines before and after each matching block.

## Install

```bash
cargo install --path .
```

## Usage

```bash
some-command 2>&1 | wlr
```

By default, `wlr` filters for red lines.

## Examples

Filter for red lines only:

```bash
some-command 2>&1 | wlr
```

Filter for all colored lines:

```bash
some-command 2>&1 | wlr --color all
```

Match multiple colors:

```bash
some-command 2>&1 | wlr --color red --color violet
```

Include context around matches:

```bash
some-command 2>&1 | wlr -B 3 -A 2
```

Disable the blank-line separator between matching sections:

```bash
some-command 2>&1 | wlr --separator ""
```

Use a custom separator between matching sections:

```bash
some-command 2>&1 | wlr --separator "\n--\n"
```
