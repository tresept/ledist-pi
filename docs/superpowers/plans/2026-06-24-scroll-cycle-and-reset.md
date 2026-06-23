# Scroll Cycle and Form Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continue an E233 scroll cycle with selected normal pages, and add a browser-only form reset button.

**Architecture:** Keep `ScriptRunner` unchanged. Extend `compile_e233` so its cyclic action list contains a scroll prefix, scroll/while-scroll actions, then selected post-scroll frames. The UI reset reuses `choose()` to reconstruct the dynamic form from the active profile and does not call an API.

**Tech Stack:** Rust, `ScriptRunner`, E233 asset planner, vanilla HTML/JavaScript, Cargo integration tests.

## Global Constraints

- Fixed pages remain three seconds as configured by the profile.
- Scroll continues while its chosen cycle frames are shown.
- No API request is made by resetting the form.
- Work is committed directly on `main` as requested by the user.

---

### Task 1: Compile post-scroll normal pages

**Files:**
- Modify: `src/e233.rs:compile_e233`
- Modify: `tests/e233.rs`

**Interfaces:**
- Consumes: `normal_pages(profile, assets, config, selection) -> Result<Vec<RgbFrame>, String>`.
- Produces: A cyclic `ScriptRunner` whose action list performs `scroll prefix -> active scroll -> post-scroll frames -> repeat`.

- [ ] **Step 1: Write the failing integration test**

Add a test that creates 48x32, 80x16, and 80x32 PNG assets; chooses destination, next stop, route, through route, service change, and scroll text; compiles the runner; and advances it past `WaitScrollEnd`. Assert that it presents the Japanese and English destination/next-stop pages, composed route/through page, and service-change page before a new scroll begins.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test e233 scroll_cycle_shows_selected_normal_pages_after_it_finishes`

Expected: FAIL because the cyclic action list ends at `WaitScrollEnd` and immediately starts the prefix again.

- [ ] **Step 3: Implement the minimal action-list extension**

After appending `ScriptAction::WaitScrollEnd` in the nonempty-scroll branch of `compile_e233`, append each post-scroll frame as `Present { scroll: None }` plus `Wait(duration)`. The post-scroll helper includes the Japanese/English destination-and-next-stop pair only when `next_stop` participates, followed by route/through and service-change pages; it does not duplicate the initial destination-only prefix.

- [ ] **Step 4: Run the focused test**

Run: `cargo test --test e233 scroll_cycle_shows_selected_normal_pages_after_it_finishes`

Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add src/e233.rs tests/e233.rs && git commit -m "Continue scroll cycles with normal pages"`

### Task 2: Add browser-only form reset

**Files:**
- Modify: `web/index.html`
- Modify: `web/app.js`

**Interfaces:**
- Consumes: `choose()` to build the active profile's initial form values.
- Produces: `#reset` button that rebuilds the form without calling a display endpoint.

- [ ] **Step 1: Add the reset button and handler**

Insert `<button id="reset" type="button">選択をリセット</button>` beside the existing controls. Implement an async `resetForm()` that awaits `choose()` and sets `選択項目をリセットしました。`; assign it to `#reset`. `choose()` establishes the asset, text, speed, brightness, and checkbox defaults.

- [ ] **Step 2: Verify no display request is made**

Run: `rg -n "id=\"reset\"|resetForm|/api/display/apply" web/index.html web/app.js`

Expected: the reset handler calls only `choose()` and `message()`; `apply` remains the sole handler that posts `/api/display/apply`.

- [ ] **Step 3: Commit**

Run: `git add web/index.html web/app.js && git commit -m "Add form selection reset button"`

### Task 3: Verify the full change

**Files:**
- Verify: `src/e233.rs`, `tests/e233.rs`, `web/index.html`, `web/app.js`

- [ ] **Step 1: Format and run all Rust verification**

Run: `cargo fmt --check && cargo test && cargo check && git diff --check`

Expected: all commands exit successfully.

- [ ] **Step 2: Confirm a clean worktree**

Run: `git status --short`

Expected: no output after the Task 1 and Task 2 commits.
