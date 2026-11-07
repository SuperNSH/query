# Finish brief: rules for all four agents

The owner's instruction: "Please finish every one of them. These are from TanStack's own code. We want all the features for NSH, same as we use in our gpui-query. Make sure to utilize the NSH skills so we build them on the standards of NSH state management."

`docs/brief-common.md` still applies: same paths, no git, no pushing, no questions, nothing weakened or skipped, and paste real output. This brief adds to it.

## Read these before writing code

From `~/fo/nsh-starter/.agents/skills/nsh/`:

- `SKILL.md`
- `references/coding-guides.md`: "Understand what runs once and what runs again", "State ownership", "Keep ownership links and subscriptions bounded", "Avoid state feedback loops", "Async work and side effects", "Public API design", "Performance rules", "Common failure modes", "Rules for coding agents"
- Rust: `references/rust.md`, "Signals", "computed and effect", "Owning state", "Async", "Stale results", "Cleanup"
- TypeScript: `references/tsx.md`, "computed or a closure", "Context", "Effects and cleanup", "resource, and stale results"
- `~/fo/.refs/nsh-v0.4.0/website/docs/reactivity.md`, in full
- `~/fo/nsh-starter/.agents/skills/nsh-testing/SKILL.md`, and `nsh-forms-navigation/SKILL.md` for forms

## NSH's state standards

NSH's rule: "a component runs once, and only the bindings that read a changed signal run again." It lists "one signal containing the app's unrelated state" as a failure mode. Measured today, this library breaks that rule, so fixing it is part of this work.

1. **A binding wakes only for what it read.** No shared version counter that every read subscribes to. A plain read method such as `query.data()`, `field.value()` or `field.is_touched()` wakes its binding only when that value changes.
2. **Signals for what is shown, `stored` for what is not.** State a binding reads is a signal, and writing an equal value wakes nothing. A cache, a running task or a timer is `stored`.
3. **Derived values are `computed`.** Never an effect that writes a second signal. An effect never writes a signal it reads.
4. **One write, one update.** Group the writes of one change in `batch`.
5. **Owners are explicit.** A signal belongs to the owner that made it, and reading one after its owner goes panics. What outlives a component, such as a cache entry, a mutation in flight or a form's metadata, lives under `create_root`. Do not create signals or effects inside a binding or a row function.
6. **Everything started ends.** Each subscribe has its stop through `cleanup`. Timers and tasks sit under an owner.
7. **Stale answers are dropped,** by a revision or an identity.
8. **Handles are `Copy`** in Rust, and are never wrapped in `Rc`. Prefer `.with` over cloning large values.
9. **No feedback loops.** Never send an owner-supplied value back through the user's callback during sync.
10. **Public API, from "Public API design".** Every export has a doc comment. Rust structs with public fields carry `#[non_exhaustive]` and implement `Default`. Options that may grow are one object. Results are objects, not tuples. A rename keeps the old name as a deprecated alias. A Promise rejects on failure, and a failure the caller can handle carries a `code`.
11. **Do not change NSH to fit this library.**

For each thing you change, you must be able to name its owner, what wakes it, and the test that fails if it regresses.

## What "finished" means

- **No row is `not done`.** Implement it and test it. "No caller", "heavy", "not exercised" and "something else fills that role" are not reasons to leave a row open.
- **A row is `not applicable` only if the concept cannot exist on NSH,** and the note names what it depends on, for example the DOM. TypeScript type-only names and core internals keep their current status. The owner reviews every `not applicable` row.
- **`done` means a test that goes through a real UI and can fail.** Check it against the TanStack source, not against what the code does. If a test shows the code differs from TanStack, fix the code and list the fix in your report.
- **A real difference from TanStack stays listed** in the parity file with its reason.

## Tests the owner wrote

These test files measure rule 1. You must make them pass **without editing them**:

- `crates/nsh-query/tests/fine_grained.rs`
- `crates/nsh-form/tests/fine_grained.rs`
- `packages/nsh-form/tests/fine_grained.test.tsx`, with the `fine` screen in `packages/nsh-form/App.tsx`

If you believe an assertion is wrong, leave it failing and say why in your report. Do not change it.

## How to report

As in `docs/brief-common.md`, plus:

1. Every code fix a test forced.
2. Every remaining `not applicable` row you changed or added, with its reason.
3. How you met the state standards: for each public read, what signal or memo it subscribes to.
