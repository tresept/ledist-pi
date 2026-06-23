# LEDist Pi

Raspberry Pi 5 とHUB75 64×32パネル2枚（128×32）用の車両側面LED表示器です。HUB75出力にはGPL-2.0-or-laterの`rust-hub75-matrix`を使用します。

## 起動

開発PCでは`data/runtime.toml`を`backend = "simulator"`または`"null"`にします。

```sh
cargo run
```

実機はPi 5で、HUB75パネルには独立した5V電源を使い、PiとGNDを共通化したうえで実行します。

```sh
sudo cargo run --release --features hardware
```

`matrix.gpio_slowdown`、`rp1_backend`、パネル固有の多重化設定は、表示が乱れる場合に`data/runtime.toml`で調整してください。

## E233系7000番台の素材

自由記述スクリプトはありません。WebUIでS（種別）、R（路線名）、C（種別変更）、T（直通先路線名）、D（行先）、スクロール文字を選ぶと、規則に従う3秒ページ列が自動生成されます。各候補には「なし」と「無表示」があります。

PNGは`data/trains/e233-9000/assets/`の以下へ配置します。ファイル名（拡張子なし）がアセットIDです。同じIDの画像を複数サイズ・言語へ置くと、表示レイアウトに合わせて使い分けます。

```text
service/128x32           128x32
service/48x32/ja         48x32
service/48x32/en         48x32
destination/128x32       128x32
destination/80x32        80x32
destination/80x16/ja,en  80x16
destination/128x16       128x16（オプション）
route/80x32,80x16        路線名
through-route/80x32,80x16 直通先路線名
route-through/80x32       路線名・直通先の併記（完成画像）
service-change/80x32     種別変更
next-stop/80x16/ja,en    次駅
next-stop/128x16/ja,en   種別なし用の次駅（オプション）
```

スクロールは東雲BDF（`data/fonts/shinonome-mincho-16/`）で描画し、種別ありなら右下80×16、種別なしなら下段128×16だけを更新します。WebUIでスクロール速度と、スクロール中に切り替える日英行先・路線名・直通先路線名を選べます。停止はその瞬間のフレームを残します。

`data/test.gif`は128×32のループGIFです。WebUIの「テストパターンの表示」で連続再生し、通常表示・停止・消灯で直ちに中断します。

## 将来車種

非E233車種は`data/trains/<id>/patterns/default.toml`にページを定義できます。各`[[page]]`へ`seconds`、`until_scroll_end`、`[[page.layer]]`（`directory`、`asset`、`x`、`y`、`width`、`height`）と任意の`[page.scroll]`を記述します。アプリ本体を変更せずにページ列を追加できます。
