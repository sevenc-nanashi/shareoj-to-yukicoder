# shareoj_to_yukicoder

ShareOJの問題ディレクトリをyukicoder向けに変換するCLI。

## やること

- `input`コードフェンスの言語指定を除去し、そのセクションに`@input`を付与します。
- サンプルを`@samples`、`@sample`、`@in`、`@out`形式に変換します。本文中の入出力と説明は保持します。
- `:::details`は`@details` / `@enddetails`に変換します。
- ローカル画像とShareOJ画像をBase64のimgタグに埋め込みます。
- tests内のテストケースをyukicoderの一括アップロード用の形式に変換します。

解説、ジャッジコード、時間・メモリ制限、アップロードは対象外です。インタラクティブ問題と独自チェッカーは手動移行が必要です。

## リリース

GitHub Actionsの`Release`をデフォルトブランチで実行し、`version`に`0.2.0`のように`v`なしのバージョンを指定します。
`Cargo.toml`の更新、`cargo update --workspace`、Linux x86_64向けビルド、更新コミットと`v<version>`タグのpush、packslipによる署名、GitHub Releaseの公開を順に実行します。
`0.2.0-rc.1`のようなバージョンはプレリリースとして公開します。

ワークフローの`GITHUB_TOKEN`によるデフォルトブランチとタグへのpushを許可する必要があります。
既存タグがある場合は停止します。push後に署名や公開が失敗した場合は、同じバージョンで再実行する前に、作成済みコミットとタグを確認してください。
