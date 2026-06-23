# Live Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** WebUIへLEDパネルの最新128×32フレームを30fpsで表示する。

**Architecture:** 表示ワーカーが`present`した`RgbFrame`を`Arc<Mutex<Option<RgbFrame>>>`へ複製する。Axumはその共有フレームをPNGにエンコードして返し、WebUIの`<img>`が約33msごとに読み込む。HUB75の非`Send`ハンドルは表示ワーカーから移動しない。

**Tech Stack:** Rust、Axum、image PNG encoder、ブラウザJavaScript。

## Global Constraints

- プレビュー画像は常に128×32、`image/png`で返す。
- 更新間隔は表示ワーカーと同じ30fps（約33ms）。
- `hardware` featureの有無に関わらず同じAPIを提供する。
- 実機では追加のPNGファイル出力を行わない。

---

### Task 1: 表示ワーカーのプレビューフレーム共有

**Files:**
- Modify: `src/display.rs`
- Modify: `src/lib.rs`
- Test: `tests/frame_and_display.rs`

**Interfaces:**
- Produces: `PreviewFrame = Arc<Mutex<Option<RgbFrame>>>`
- Produces: `spawn_display_worker_with_preview(create, preview) -> Sender<DisplayCommand>`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn present_updates_the_shared_preview_frame() {
    let preview = new_preview_frame();
    let sender = spawn_display_worker_with_preview(
        || Ok(Box::new(NullBackend::default())), preview.clone()
    ).unwrap();
    sender.send(DisplayCommand::Present(RgbFrame::solid(128, 32, [1, 2, 3]))).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(preview.lock().unwrap().as_ref().unwrap().pixel(0, 0), Some([1, 2, 3]));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test frame_and_display present_updates_the_shared_preview_frame`

Expected: FAIL because preview worker interface does not exist.

- [ ] **Step 3: Implement the shared preview worker path**

```rust
pub type PreviewFrame = Arc<Mutex<Option<RgbFrame>>>;
pub fn new_preview_frame() -> PreviewFrame { Arc::new(Mutex::new(None)) }
// Set `*preview.lock().unwrap() = Some(frame.clone())` immediately before
// every backend `present`, including script and GIF frames.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test frame_and_display present_updates_the_shared_preview_frame`

Expected: PASS.

### Task 2: PNG preview endpoint and 30fps WebUI refresh

**Files:**
- Modify: `src/web.rs`
- Modify: `src/main.rs`
- Modify: `web/index.html`
- Modify: `web/app.js`
- Test: `tests/web_api.rs`

**Interfaces:**
- Consumes: `PreviewFrame`
- Produces: `GET /api/display/preview.png`

- [ ] **Step 1: Write the failing endpoint test**

```rust
#[tokio::test]
async fn preview_endpoint_returns_a_128_by_32_png() {
    let response = app.oneshot(Request::get("/api/display/preview.png").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
    let image = image::load_from_memory(&body).unwrap();
    assert_eq!(image.dimensions(), (128, 32));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test web_api preview_endpoint_returns_a_128_by_32_png`

Expected: FAIL with HTTP 404.

- [ ] **Step 3: Implement endpoint and UI**

```rust
async fn preview_png(State(state): State<Arc<AppState>>) -> Response {
    // clone latest frame or RgbFrame::black(128, 32), then PNG encode
}
```

```js
setInterval(() => {
  preview.src = `/api/display/preview.png?t=${Date.now()}`;
}, 1000 / 30);
```

- [ ] **Step 4: Run endpoint and full test suite**

Run: `cargo fmt --check && cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/display.rs src/web.rs src/main.rs src/lib.rs web/index.html web/app.js tests/frame_and_display.rs tests/web_api.rs
git commit -m "Add 30fps live display preview"
```
