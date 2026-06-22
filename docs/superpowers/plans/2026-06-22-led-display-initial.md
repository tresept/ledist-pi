# HUB75 車両側面LED表示器 初期版 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Raspberry Pi 5の128×32 HUB75表示器を、TOMLプロファイル、PNGアセット、DSL、動的WebUIで制御できるRustアプリケーションとして実装する。

**Architecture:** `core`（プロファイル、アセット、フレーム、DSL）をHTTPと実機出力から分離する。表示スレッドはコマンドチャネルで置換可能なプログラムを受け取り、`Null`、PNGファイル出力の`Simulator`、`rust-hub75-matrix`の`Matrix`バックエンドへ完成RGBフレームを送る。Axumはプロファイル内容とアセット一覧を提供し、適用前に全入力とDSLを検証する。

**Tech Stack:** Rust 2024, Axum, Tokio, Serde/TOML, image, notify, rust-hub75-matrix, vanilla HTML/JavaScript.

## Global Constraints

- キャンバスは既定で128×32 RGB8、`frame ... end`は完成フレームを一度だけpresentする。
- アセットは起動時に登録したIDからのみ解決し、リクエスト文字列をパスへ連結しない。
- 適用の失敗は実行中プログラムを置換しない。
- 実機バックエンドは`rust-hub75-matrix`を利用し、Pi以外はNullまたはSimulatorで検証する。

### Task 1: プロジェクト基盤とフレーム合成

**Files:** `Cargo.toml`, `src/lib.rs`, `src/frame.rs`, `src/display.rs`, `tests/frame_and_display.rs`

- [ ] フレームの境界クリッピング、黒消去、PNG合成、Null/Simulatorバックエンドの失敗テストを先に書く。
- [ ] RGB8フレームとバックエンドtraitを実装し、テストを緑にする。

### Task 2: プロファイル、アセット、および入力検証

**Files:** `src/profile.rs`, `src/assets.rs`, `tests/profile_and_assets.rs`, `data/trains/*/profile.toml`

- [ ] TOML読込、領域検証、ファイル名サニタイズ、PNG寸法検証、ID解決の失敗テストを先に書く。
- [ ] レジストリとプロファイル検証を実装し、E233/E353のサンプルを追加する。

### Task 3: DSLと表示ランナー

**Files:** `src/dsl.rs`, `src/runner.rs`, `tests/dsl_and_runner.rs`

- [ ] frame原子更新、wait、loop、scroll_end、不明IDの行番号付きエラーの失敗テストを先に書く。
- [ ] パーサ、静的検証、経過時間ベースのランナーを実装する。

### Task 4: HTTP APIと動的WebUI

**Files:** `src/web.rs`, `src/main.rs`, `web/index.html`, `web/app.js`, `tests/web_api.rs`

- [ ] プロファイル取得、テンプレート取得、表示適用、失敗時の状態維持、消灯の失敗テストを先に書く。
- [ ] Axum API、表示コマンドチャネル、TOML runtime保存、動的フォーム画面を実装する。

### Task 5: 実機バックエンド、データ一式、統合検証

**Files:** `src/display.rs`, `data/runtime.toml`, `data/trains/**`, `README.md`, `tests/integration.rs`

- [ ] Matrix設定の変換と安全なRGBバッファ長の失敗テストを先に書く。
- [ ] `rust-hub75-matrix`実装、テンプレート、スクロール既定値、実行手順を追加する。
- [ ] `cargo test`、`cargo clippy -- -D warnings`、HTTPスモークテストを実行する。
