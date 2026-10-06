# Contributing to Gatekeeper

Thank you for your interest in contributing to Gatekeeper! We welcome community feedback, forks, discussions, and contributions from developers, security researchers, and GLAM institutions (Galleries, Libraries, Archives, and Museums).

---

## Contribution Workflow

### 1. Discuss Before Submitting Code
To maintain architectural coherence, security integrity, and high throughput performance, **please open an Issue or Discussion before opening a Merge/Pull Request**.

> [!IMPORTANT]
> **Uncommented or unannounced Merge/Pull Requests without prior context will be closed without review.**  
> We value your time—discussing architectural changes, new features, or refactorings beforehand prevents wasted effort on implementations that might conflict with our threat model or deployment constraints.

### 2. Forking and Experimentation
Forking the repository to adapt Gatekeeper to your institution's specific infrastructure, run experiments, or prototype new features is explicitly welcome under the Apache 2.0 license. If you develop improvements that could benefit the broader community, please share your findings with us!

### 3. Reporting Bugs and Issues
When reporting a bug or submitting feedback:
* Check existing issues first to avoid duplicates.
* Provide a clear description of the observed behavior, steps to reproduce, and the expected outcome.
* Include relevant environment details (operating system, NGINX version, Rust toolchain version).
* **Security & Privacy:** Never post private cryptographic keys (`app_key`), internal tokens, or sensitive production IP logs.

---

## Development & Code Guidelines

If your proposal has been discussed and you are preparing a contribution:

1. **Keep Changes Focused and Atomic:**
   * One feature, improvement, or bug fix per request.
   * Avoid bundling unrelated refactorings or dependency updates with functional changes.
2. **Intent Over Syntax:**
   * Code comments and commit descriptions should explain the *why* (design decisions, edge cases, performance trade-offs) rather than merely restating what the syntax does.
3. **Verify Tests and Release Builds:**
   * All unit tests must pass:
     ```bash
     cargo test
     ```
   * The release binary must compile without warnings or errors:
     ```bash
     cargo build --release
     ```
4. **Code Standards:**
   * Keep all technical terminology, variable names, functions, documentation, and commit messages in English.
   * Format code using standard Rust tooling (`cargo fmt` and `cargo clippy`).

---

## License

By submitting contributions to Gatekeeper, you agree that your code will be licensed under the project's [Apache License 2.0](./LICENSE.txt).
