# RFC 0002 - A simple authoring syntax for Lightdown

## Status

Draft.

## Summary

RFC 0001 defines Lightdown IR as the structural representation of a document.
This RFC defines a simple author-facing Lightdown syntax that maps to that IR.

The authoring syntax is intentionally small and Markdown-like. It is designed
for common technical documentation tasks such as:

- headings;
- paragraphs;
- lists;
- block quotes;
- fenced code blocks;
- thematic breaks;
- emphasis and strong emphasis;
- inline code;
- links and images;
- explicit inline escape hatches using embedded IR.

The result of parsing a Lightdown document is always a `doc` node in RFC 0001
IR version `0.1.0`.

## Goals

The initial Lightdown authoring syntax should satisfy the following goals:

- It should be easy to write by hand.
- It should be easy to read in plain text form.
- It should map to RFC 0001 IR in a direct and predictable way.
- It should cover the common structures used in technical documentation.
- It should provide an explicit escape hatch for constructs that are easier to
  express directly in IR.

## Non-goals

This RFC intentionally keeps the surface syntax small.

- It does not define a full parsing algorithm.
- It does not define every corner case of Markdown compatibility.
- It does not guarantee round-tripping from arbitrary IR back to authoring
  syntax.
- It does not define layout-oriented syntax.
- It does not define an extension registry yet.

## Overview

A Lightdown source document is converted into a Lightdown IR document:

```lightdown
# Hello

Lightdown is small.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (h1 "Hello")
  (p "Lightdown is small."))
```

The generated `doc` node must always carry:

```lightdown-ir
{:meta {:version "0.1.0"}}
```

This RFC defines block syntax, inline syntax, and the mapping rules for each
construct.

## Block syntax

### Headings

ATX headings use one to six `#` characters followed by a space.

```lightdown
# Title
## Section
### Subsection
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (h1 "Title")
  (h2 "Section")
  (h3 "Subsection"))
```

The number of `#` characters determines the target node:

- `#` maps to `h1`;
- `##` maps to `h2`;
- `###` maps to `h3`;
- `####` maps to `h4`;
- `#####` maps to `h5`;
- `######` maps to `h6`.

The heading content is parsed as inline content.

### Paragraphs

One or more non-empty lines that do not start another block form a paragraph.
Adjacent paragraph lines are joined into the content of a single `p` node.

```lightdown
Lightdown is a small markup language.
It maps into a structural IR.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p
    "Lightdown is a small markup language. "
    "It maps into a structural IR."))
```

This RFC does not require a specific whitespace-normalization algorithm, but an
implementation should preserve ordinary reading flow rather than treating
adjacent paragraph lines as separate blocks.

### Unordered lists

Unordered list items start with `- `.

```lightdown
- Small
- Predictable
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (ul
    (li (p "Small"))
    (li (p "Predictable"))))
```

Each authoring list item maps to an `li` node. In the simplest case, its text
content maps to a single paragraph inside that `li`.

### Ordered lists

Ordered list items start with a decimal number followed by `. `.

```lightdown
1. Parse the document.
2. Build the IR.
3. Render HTML.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (ol
    (li (p "Parse the document."))
    (li (p "Build the IR."))
    (li (p "Render HTML."))))
```

### Block quotes

Quoted blocks use `>` at the beginning of the line.

```lightdown
> Programs should be written for people to read.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (blockquote
    (p "Programs should be written for people to read.")))
```

### Fenced code blocks

Code fences use triple backticks. An optional language name may follow the
opening fence.

~~~lightdown
```rust
fn main() {
    println!("hello");
}
```
~~~

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (codeblock {:lang "rust"} """
    | fn main() {
    |     println!("hello");
    | }
    """))
```

If no language name is present, the block maps to `codeblock` without `:lang`.

### Thematic breaks

A line containing three hyphen-minus characters forms a thematic break.

```lightdown
---
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (hr))
```

## Inline syntax

### Plain text

Ordinary text maps to string children in inline position.

```lightdown
Lightdown is plain text.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p "Lightdown is plain text."))
```

### Emphasis

Single asterisks mark emphasis.

```lightdown
Use *simple* data structures.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p "Use " (em "simple") " data structures."))
```

### Strong emphasis

Double asterisks mark strong emphasis.

```lightdown
Prefer **explicit** rules.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p "Prefer " (strong "explicit") " rules."))
```

### Inline code

Backticks mark inline code.

```lightdown
Run `lightdown build`.
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p "Run " (code "lightdown build") "."))
```

### Links

The standard authoring syntax for links is:

```text
[label](https://example.com)
```

For example:

```lightdown
Read [the guide](https://example.com/guide).
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p
    "Read "
    (a {:href "https://example.com/guide"} "the guide")
    "."))
```

The link label is parsed as inline content.

### Images

Images use the common Markdown-like form:

```text
![alt text](https://example.com/image.png)
```

For example:

```lightdown
![Architecture overview](https://example.com/overview.png)
```

maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (p
    (img
      {:src "https://example.com/overview.png"
       :alt "Architecture overview"})))
```

## Embedded IR

Some inline constructs are easier to express directly in Lightdown IR than in
surface syntax. For those cases, Lightdown provides embedded IR.

Embedded IR starts with `\(` and ends with the matching `)`.

The content inside `\(... )` must be a valid RFC 0001 IR expression in the
target context.

For example:

```lightdown
\(code "lightdown")
```

inside a paragraph maps to:

```lightdown-ir
(code "lightdown")
```

### Embedded Lightdown fragments inside IR

Within embedded IR, square brackets introduce a nested Lightdown fragment.
That fragment is parsed as Lightdown authoring syntax and then lowered to IR
before being inserted into the surrounding embedded IR expression.

This mechanism is intended for cases where most of a construct is easier to
write in IR, but part of its child content is still more naturally written in
Lightdown syntax.

For example:

```lightdown
\(a {:href "https://example.com"} [`lightdown`])
```

maps to:

```lightdown-ir
(a {:href "https://example.com"} (code "lightdown"))
```

In the initial version of this syntax:

- embedded IR is primarily intended for inline position;
- a bracketed fragment inside embedded IR must lower to content that is valid
  in the surrounding IR position;
- an implementation should reject embedded IR that becomes invalid after
  lowering nested Lightdown fragments.

## Mapping rules

The mapping from authoring syntax to IR follows these general rules:

- every document maps to a single `doc` node with `{:meta {:version "0.1.0"}}`;
- every block construct maps to one RFC 0001 block-level node;
- every inline construct maps to RFC 0001 inline content;
- list item text maps to paragraph children inside `li`;
- link destinations map to `a {:href ...}`;
- image destinations and alt text map to `img {:src ... :alt ...}`;
- fenced code blocks map to `codeblock`, with the fence language mapped to
  `:lang` when present;
- embedded IR maps directly to its RFC 0001 node form after nested Lightdown
  fragments, if any, are lowered.

## Validity rules

An implementation should reject invalid source instead of guessing.

At minimum, the following cases are invalid:

- a heading marker without heading content;
- an unterminated code fence;
- malformed link or image syntax;
- malformed embedded IR;
- an embedded IR expression that is not valid RFC 0001 IR;
- a bracketed Lightdown fragment inside embedded IR that lowers to content not
  allowed in that IR position.

## Example

The following example shows a small but representative document:

```lightdown
# Foobar

## Barfoo

Do you know \(a {:href "https://example.com"} [`lightdown`])? `lightdown` is good.
```

It maps to:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (h1 "Foobar")
  (h2 "Barfoo")
  (p
    "Do you know "
    (a {:href "https://example.com"} (code "lightdown"))
    "? "
    (code "lightdown")
    " is good."))
```
