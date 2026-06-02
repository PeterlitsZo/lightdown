# RFC 0001 - A simple Lightdown IR

## Status

Draft.

## Summary

Lightdown IR is the intermediate representation of the Lightdown markup
language. It is designed for writing documents in a Markdown-like authoring
style while keeping the structure explicit enough to convert to HTML with very
little ambiguity.

This RFC defines the first version of the IR, including:

- the document model;
- the textual representation;
- the core block-level and inline elements;
- the expected mapping from Lightdown IR to HTML.

## Goals

The initial IR should satisfy the following goals:

- It should be simple to parse and serialize.
- It should preserve document structure instead of storing only formatted text.
- It should map to HTML in a direct and predictable way.
- It should cover the common elements needed for technical documentation, such
  as headings, paragraphs, lists, links, emphasis, and code blocks.

## Non-goals

This RFC does not try to solve everything at once.

- It does not aim to represent every possible HTML feature.
- It does not define layout-oriented constructs such as grids or columns.
- It does not define an extension mechanism yet.
- It does not require round-tripping from arbitrary HTML back into Lightdown.

## Document model

The root node is `doc`.

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (h1 "Hello World")
  (p "How do you do?"))
```

The `doc` node must specify document metadata in its attribute map. The
metadata must contain the IR version with the `:version` key, whose value is a
semantic version.

All children of `doc` must be block-level elements.

## Textual representation

The textual form of Lightdown IR uses an S-expression-like syntax.

- A node is written as `(name ...)`.
- Many nodes, especially HTML-like nodes, write attributes as a map literal
  placed immediately after the node name, such as
  `(a {:href "https://example.com"} "the example")`.
- A node that uses an attribute map places that map before child nodes or
  inline text.
- In version `0.1.0`, attribute map keys are keywords such as `:href`.
  Attribute map values are strings, except for the `doc` node's `:meta` value,
  which is a nested map.
- Only nodes whose definitions explicitly mention an attribute map may use one.
- For each node, only the attributes listed in its definition are recognized.
  Unrecognized attributes must be rejected.
- Text is represented as a string.
- Lightdown IR supports both single-line strings such as `"hello"` and
  multi-line strings delimited by `"""`.
- String escaping follows the JSON string escaping rules.

For example:

```lightdown-ir
(p
  "See "
  (a {:href "https://example.com"} "the example")
  " for more details.")
```

In the initial version of the IR, strings are plain text nodes. A renderer is
responsible for escaping them correctly when producing HTML.

Multi-line strings are intended for content such as code blocks. They preserve
embedded newlines. To avoid indentation noise in indented documents, a line in
a multi-line string may begin with `|`. When it does, the indentation before
the `|` and the `|` itself are not part of the resulting string value.
An intentionally blank line may therefore be written as a line that contains
only `|`.

In version `0.1.0`, the normalization rule for multi-line strings is:

1. Split the string into lines.
2. Ignore lines whose trimmed content is empty.
3. For each remaining line, if it begins with optional whitespace, followed by
   `|`, followed by an optional single space, remove that prefix.
4. Join the resulting lines using `\n`.

For example:

```lightdown-ir
(codeblock """
  | def main():
  |   return
  """)
```

The resulting string value is:

```text
def main():
  return
```

For example, a preserved blank line may be written like this:

```lightdown-ir
(codeblock """
  | foo
  |
  | bar
  """)
```

The resulting string value is:

```text
foo

bar
```

## Supported elements

This section defines the core element set for Lightdown IR `0.1.0`.

### Block-level elements

#### Headings

Lightdown IR supports six heading levels:

- `h1`
- `h2`
- `h3`
- `h4`
- `h5`
- `h6`

Each heading contains inline content.

Example:

```lightdown-ir
(h2 "Installation")
```

#### Paragraphs

Paragraphs are represented with `p`. A paragraph contains inline content.

Example:

```lightdown-ir
(p "Lightdown aims to stay small and predictable.")
```

#### Lists

Unordered lists use `ul`, ordered lists use `ol`, and list items use `li`.

- `ul` contains one or more `li` nodes.
- `ol` contains one or more `li` nodes.
- `li` contains one or more block-level elements.

This design makes nested lists and multi-paragraph list items explicit.

Example:

```lightdown-ir
(ul
  (li
    (p "Install the CLI."))
  (li
    (p "Write a document.")
    (ul
      (li (p "Add headings."))
      (li (p "Add code blocks.")))))
```

#### Block quotes

Quoted blocks use `blockquote`. A block quote contains one or more block-level
elements.

Example:

```lightdown-ir
(blockquote
  (p "Programs should be written for people to read."))
```

#### Code blocks

Code blocks use `codeblock`.

- `codeblock` contains a single text node, which will usually be a multi-line
  string.
- `codeblock` may specify `:lang` in its attribute map to indicate the source
  language.

Example:

```lightdown-ir
(codeblock {:lang "rust"} """
  | fn main() {
  |     println!("hello");
  | }
  """)
```

#### Thematic breaks

Thematic breaks use `hr`.

Example:

```lightdown-ir
(hr)
```

#### Tables

To support common documentation use cases, the initial IR also includes tables.

- `table` contains zero or one `thead`, followed by zero or one `tbody`.
- `thead` contains one or more `tr`.
- `tbody` contains one or more `tr`.
- `tr` contains one or more `th` or `td`.
- `th` and `td` contain inline content.

Example:

```lightdown-ir
(table
  (thead
    (tr
      (th "Name")
      (th "Description")))
  (tbody
    (tr
      (td "Lightdown")
      (td "A lightweight document language"))))
```

### Inline elements

#### Plain text

Strings are inline text nodes.

Example:

```lightdown-ir
(p "This is plain text.")
```

#### Emphasis and strong emphasis

Emphasis uses `em`. Strong emphasis uses `strong`. Both contain inline content.

Example:

```lightdown-ir
(p "Use " (em "simple") " data structures and " (strong "explicit") " rules.")
```

#### Inline code

Inline code uses `code`. It contains a single text node.

Example:

```lightdown-ir
(p "Run " (code "lightdown build") " to render the document.")
```

#### Links

Links use `a`.

- `a` must have the `:href` entry in its attribute map.
- `a` contains inline content.
- `a` must not contain another `a` node as a descendant.

Example:

```lightdown-ir
(p "Read the " (a {:href "/guide/getting-started.html"} "getting started guide") ".")
```

#### Images

Images use `img`.

- `img` must have the `:src` entry in its attribute map.
- `img` should have the `:alt` entry in its attribute map.
- `img` has no child nodes.

Example:

```lightdown-ir
(p
  "Architecture overview: "
  (img {:src "/images/overview.png" :alt "Architecture overview"}))
```

#### Line breaks

Explicit line breaks use `br`.

Example:

```lightdown-ir
(p "First line." (br) "Second line.")
```

## HTML mapping

The primary motivation of this IR is straightforward conversion to HTML. The
mapping rules are intentionally simple:

- `doc` maps to an HTML document fragment, not an HTML element by itself.
- `doc` metadata does not map directly to any HTML element.
- `h1` through `h6` map directly to `<h1>` through `<h6>`.
- `p`, `ul`, `ol`, `li`, `blockquote`, `hr`, `table`, `thead`, `tbody`, `tr`,
  `th`, `td`, `em`, `strong`, `code`, `a`, `img`, and `br` map directly to
  their HTML counterparts.
- `codeblock` maps to `<pre><code>...</code></pre>`.
- `codeblock {:lang "rust"} ...` should be rendered as a language hint on the
  HTML `<code>` element, for example via `class="language-rust"`.

An implementation may wrap the rendered fragment in `<html>`, `<head>`, and
`<body>`, but that behavior is outside the scope of this RFC.

## Validity rules

An implementation should reject invalid IR instead of guessing.

At minimum, the following cases are invalid:

- a `doc` node without `:meta`;
- a `doc` node whose `:meta` map does not contain `:version`;
- a `doc` node whose `:meta` map contains unrecognized entries;
- a block-level child placed where only inline content is allowed;
- a `codeblock` node with zero or multiple children;
- a `codeblock` node whose child is not a text node;
- an `a` node without `:href` in its attribute map;
- an `a` node that contains another `a` node as a descendant;
- an `img` node without `:src` in its attribute map;
- an `img` node with child nodes.

## Example

The following example shows a small but representative document:

```lightdown-ir
(doc {:meta {:version "0.1.0"}}
  (h1 "Lightdown")
  (p
    "Lightdown is a markup language for writing documents that can be converted "
    "to HTML easily.")
  (h2 "Features")
  (ul
    (li (p "Headings"))
    (li (p "Lists"))
    (li (p "Code blocks")))
  (h2 "Example")
  (codeblock {:lang "javascript"} """
    | console.log('hello');
    """)
  (p
    "See " (a {:href "https://example.com"} "the project page") "."))
```
