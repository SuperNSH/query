# Rules for every agent on the TanStack work

Read this completely before your own brief. It applies to all four agents working on `nsh-query` and `nsh-form`.

## Where you may work

- You work in a git worktree on branch `feat/tanstack`, under `~/fo/.worktrees/`. Another agent shares your worktree and owns the other language, so edit only the paths your brief names.
- Never edit `~/fo/nsh-query`, `~/fo/nsh-form` or anything else outside your paths. Four other agents are building `nsh-starter`, `nsh-firebase`, `nsh-http` and `nsh-auth` on `main` of these libraries right now, and must not see your changes.
- Do not run any git command. Whoever started you checks and commits your work.
- Do not publish anything, and do not push.
- Do not stop to ask questions. Make the decision the brief implies, write it down in your parity file, and continue.

## References, all read-only

| Path | What it is |
|---|---|
| `~/fo/.refs/tanstack-query` | TanStack Query source. `packages/query-core/src` is the behavior, `packages/solid-query/src` is the closest adapter to NSH, because NSH's reactive graph uses Solid's algorithm |
| `~/fo/.refs/tanstack-form` | TanStack Form source. `packages/form-core/src` is the behavior, `packages/solid-form/src` is the closest adapter |
| `~/fo/.refs/gpui-query` | Our `gpui-query`. Rust ports reuse its GPUI-free `core`. `crates/gpui-query/src/hook` shows how its hooks use that core |
| `~/fo/.refs/gpui-form` | Our `gpui-form`. `docs/form-feature-parity-plan.md` compares it with TanStack Form and react-hook-form |
| `~/fo/.refs/nsh-v0.4.0` | The NSH source that is installed, and its `examples/` |
| `~/fo/nsh-starter/.agents/skills/` | NSH skills. Read `nsh`, `nsh-testing` and `nsh-forms-navigation` before writing NSH code |

Never invent an API. Look the name up, in the source for the installed version.

## Measured facts about nsh 0.4.0

Each of these came from a real run. Trust them over the docs.

- `If` and `Each` are globals in TSX. Importing them from `nsh` is a type error.
- `nsh check` needs an `App.tsx` at the package root. `.nsh/` is generated and ignored.
- `nsh check` and `nsh build` catch different things. Run both.
- A test opens an app with `open("../App.tsx", { clock: {} })`. On a manual clock, `app.clock!.runFor(ms)` moves time at once. Use it for every test with a timer, retry or debounce.
- Test network routes are `{ "GET https://host/path": { body, status, delay } }`, or a function of the request. A request no route matches fails.
- Selectors: `text:`, `id:`, `label:`, `role:button[Name]`. A step needs exactly one match, and `>> nth=0` narrows. Selector regexes use Rust's `regex` syntax, so no look-ahead.
- Annotate host-handler parameters and any callback option typed with `unknown`, or `nsh check` reports an implicit `any`.
- `Headless::start` can return before the first frame, so call `wait_for` first.
- `Headless::shows` takes plain text. `wait_for` and `press` take selectors. `wait_for_gone` also takes a timeout. `screen().count` returns a `Result`.
- `Signal::update` needs `T: Clone`. For a value with no `Clone`, use the `Tracked` cell at `crates/nsh-query/src/tracked.rs`.
- A component's signals are already disposed when its `cleanup` runs, so reading one there panics. Capture what cleanup needs in an `Rc<Cell<..>>`.
- `create_root` gives an owner that outlives the component. Use it for anything that must survive the component.
- Dev-dependencies must be `0.4.0`. The docs' `0.1` samples are stale. All NSH crates must come from one place.
- `nsh_core::time::unix_ms()` follows the engine's clock, including a manual one. `std::time` does not.
- `solid-js` cannot be imported, because the bundler refuses it. `nsh` has `signal`, `computed`, `effect`, `batch`, `untracked`, `cleanup` and `context`.

## Measured facts about TanStack on NSH

A real test ran on 0.4.0 on 2026-11-07. It used `@tanstack/query-core` 5.107.1 and `@tanstack/form-core` 1.33.5 unmodified, under QuickJS, through `nsh test`:

- `QueryObserver`, `InfiniteQueryObserver`, `MutationObserver`, `QueryClient`, retries, abort signals, `FormApi` and `FieldApi` all worked, on a manual clock.
- `focusManager.isFocused()` and `onlineManager.isOnline()` both reported `true`.
- A release build of both cores plus a small app is 80.7 KB of JS and 91.9 KB of bytecode.

## Code rules

- Comments are 1-2 lines and state a non-obvious constraint. Never narrate the next line.
- Public items have a short doc comment saying what callers get and any preconditions.
- Dead code is deleted, and there are no speculative abstractions.
- Docs read like a competent person wrote them: direct and concrete, with no marketing filler.
- Tests go through a real UI on a manual clock, with no real network and no real time.
- Every new test must be able to fail. Break the code on purpose once per feature group, confirm the test fails, and put it back.
- Never weaken, skip or delete a check to get a pass. If something cannot be done, mark it `not done` and say why.

## How to report

Your final message must contain these things, and nothing that was not measured:

1. The last lines of each gate command's real output, pasted as they were printed.
2. The number of tests that pass.
3. Every row of your parity file that is not `done`, with the reason.
4. Anything you could not verify.

If you run out of room before finishing, stop at a state where the gates pass, and list exactly what remains. Claiming done without the pasted output counts as not done.
