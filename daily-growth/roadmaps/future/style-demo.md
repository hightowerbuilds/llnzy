# The Quiet Rise of Terminal-First Development

This document demos the markdown preview styles. In Newspaper it should read like an article page; in Research Paper it should read like an arXiv preprint, with a centered unbolded title and numbered sections.

## Background

Developers increasingly organize their entire workflow around a terminal session, with editors, file trees, and prompt queues orbiting the shell rather than the other way around. This paragraph is long enough to wrap several times so you can judge the body typeface, measure, and leading of each style at a glance.

### Early influences

The lineage runs from teletypes through xterm to GPU-accelerated emulators, each generation trading a little compatibility for a lot of speed.

## Method

We evaluate three preview styles against two print references and one native baseline.

- Body typeface and measure
- Heading hierarchy and section rules
- Quote and code treatments

> Abstract-like blockquote: in Research Paper style this should be set smaller with symmetric indentation and no bar; in Newspaper style it should be an italic pull-quote with a rule.

```
fn evaluate(style: MarkdownPreviewStyle) -> Verdict {
    Verdict::Ship
}
```

### Threats to validity

A single reader, one window width, and the absence of real inline formatting in the parser.

## Conclusion

Switch styles in Appearances → Editor → Preview Style and watch this file change character completely.
