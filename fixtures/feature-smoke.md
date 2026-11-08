---
title: mdoc feature smoke test
---

# mdoc

This paragraph contains **bold**, *italic*, `inline code`, ~~removed text~~,
and an [external link](https://example.com).

## Lists

- [x] Parse Markdown
- [ ] Improve DOCX round trips

1. First
2. Second

> A block quote with **formatting**.

```rust
fn main() {
    println!("hello");
}
```

| Feature | Status |
| :--- | ---: |
| CommonMark | Working |
| DOCX export | Initial |

Inline math: $E = mc^2$.

Reference to a footnote.[^note]

[^note]: Footnote content.
