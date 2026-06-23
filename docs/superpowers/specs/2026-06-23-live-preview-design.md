# 128×32リアルタイムプレビュー設計

## 目的

WebUIに、LEDパネルへ最後に送られた128×32 RGBフレームをリアルタイム表示する。実機と表示内容を一致させ、`hardware` featureを持たない開発環境でも同じAPIで確認できるようにする。

## 構成

- 表示ワーカースレッドが`present`直前の`RgbFrame`を共有プレビュー状態へ複製する。
- `GET /api/display/preview.png`はその最新フレームをPNGとして返す。まだフレームがなければ黒い128×32画像を返す。
- WebUIは`<img width="128" height="32">`を設置し、キャッシュ回避用の時刻クエリ付きURLを30fps（約33ms）で更新する。
- `blank`、通常ページ、GIFの各フレームも同じプレビュー状態を更新する。停止は現フレームを保持する。

## feature分岐

- `hardware` featureあり: MatrixBackendへ送った実フレームを共有する。PNGファイルを書き出さない。
- `hardware` featureなし: NullBackend/SimulatorBackendへ送った同一フレームを共有する。Simulatorの出力先PNGは補助デバッグ用途に留める。

## エラー処理と検証

- PNG応答は常に128×32、`image/png`とする。
- APIテストで通常表示、消灯、未表示時の黒フレームを確認する。
- 表示ワーカーの`Matrix`所有権を動かさず、`RgbFrame`の複製だけをスレッド間共有する。
