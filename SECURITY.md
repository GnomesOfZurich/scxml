# Security Policy

## Reporting a vulnerability

If you discover a security vulnerability in `scxml`, please **do not** open a
public GitHub issue. Instead, email **security@gnomes.ch** with:

- A description of the issue and its potential impact.
- Steps to reproduce, ideally with a minimal SCXML payload.
- The crate version, Rust version, and target platform.
- Whether you are willing to be credited in the public advisory.

We will acknowledge receipt within 48 hours and aim to provide a fix or
mitigation timeline within 7 days. Once a fix is published, we will credit
reporters in the release notes unless they prefer to remain anonymous.

## Supported versions

Security fixes are provided for the **latest published minor version** of the
crate. Older versions are not patched. Users are encouraged to track the
current release.

## Threat model

The crate is designed to safely parse SCXML from untrusted sources via the
[`parse_untrusted()`](https://docs.rs/scxml/latest/scxml/sanitize/fn.parse_untrusted.html)
entry point. Threats actively mitigated:

| Threat | Mitigation |
|--------|------------|
| XXE / billion laughs | `quick-xml` does not expand entities. `parse_untrusted` rejects `<!DOCTYPE` and `<!ENTITY` outright. |
| Memory exhaustion (oversized input) | Configurable `max_input_bytes`, `max_states`, `max_transitions` limits. |
| Stack overflow (deep nesting) | Configurable `max_depth` (default 20). |
| Identifier injection | All identifiers validated against `[a-zA-Z0-9_\-\.:]`. |
| Export injection | XML attribute escaping, DOT string escaping, Mermaid sanitisation. |
| Code execution | The `Statechart` model is inert data. Guards are string names, never evaluated. `<script>` and `<invoke>` are stored as descriptors, never executed. `#![forbid(unsafe_code)]` is enforced. |

The full security model is documented in the [README](README.md#security-model).

## Out of scope

The following are **not** considered vulnerabilities in this crate:

- Misuse of `parse_xml()` (the unchecked parser) on untrusted input; use
  `parse_untrusted()` instead.
- Bugs in downstream consumers of the model (e.g. a calling application that
  evaluates a guard string as JavaScript).
- Denial of service from a caller setting deliberately permissive
  `InputLimits`.
- Issues in Graphviz, Mermaid, or other tools that consume our exporter
  output.
