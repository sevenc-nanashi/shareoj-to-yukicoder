# shareoj-to-yukicoder

ShareOJの問題ディレクトリをyukicoder向けに変換するCLI。

## インストール

```bash
packslip install github.com/sevenc-nanashi/shareoj-to-yukicoder --pin ps1_3qycyvage4w6vkt4urnd533azq
```

## やること

- `input`コードフェンスの言語指定を除去し、そのセクションに`@input`を付与します。
- サンプルを`@samples`、`@sample`、`@in`、`@out`形式に変換します。本文中の入出力と説明は保持します。
- `:::details`は`@details` / `@enddetails`に変換します。
- ローカル画像とShareOJ画像をBase64のimgタグに埋め込みます。
- tests内のテストケースをyukicoderの一括アップロード用の形式に変換します。

解説、ジャッジコード、時間・メモリ制限、アップロードは対象外です。インタラクティブ問題と独自チェッカーは手動移行が必要です。

## ライセンス

MIT Licenseで公開しています。
