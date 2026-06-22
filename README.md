# LEDist Pi

Raspberry Pi 5 とHUB75 64×32パネル2枚（128×32）向けの車両側面LED表示器です。GPIO/HUB75出力にはGPL-2.0-or-laterの`rust-hub75-matrix`を使用します。配布時にはアプリケーションも含めてライセンス条件を確認してください。

## 起動

開発PCでは`data/runtime.toml`の`backend = "simulator"`または`"null"`を使います。

```sh
cargo run
```

`http://localhost:3000`を開きます。実機では`backend = "matrix"`に変更し、Pi 5上で次を実行します。

```sh
sudo cargo run --release --features hardware
```

LEDパネルには独立した5V電源を使用し、PiとGNDを共通化してください。

## 車種と素材の追加

`data/trains/<profile-id>/profile.toml`、`assets/`、`templates/`だけで車種を拡張できます。`asset_dir`配下のPNG名（拡張子なし）がWebUI候補名です。`require_exact_size=true`のPNGは`target_region`と同一寸法でなければ適用できません。Unicode対応BDFは`data/fonts/<font-id>/font.bdf`へ配置します。

Piで表示が乱れる場合は、`matrix.gpio_slowdown`、`rp1_backend`、パネル固有の多重化設定を調整してください。
