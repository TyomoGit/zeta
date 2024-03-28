# Zeta

ZennとQiitaの記事を管理するツール

## インストール
[Cargo](https://www.rust-lang.org/ja/tools/install)が必要です。
```sh
cargo install --git https://github.com/TyomoGit/zeta.git
```

## 使い方
任意のディレクトリで初期化を行う
```sh
zeta init
```

GitHubリポジトリを作成し、ZennのGitHub連携とQiitaのアクセストークンの登録を完了する

記事を作成する
```sh
zeta new my-article-name
```
`zeta/`ディレクトリにファイル`my-article-name.md`が作成される

記事を書く

ビルトする
```sh
zeta build my-article-name
```

mainブランチにプッシュで公開する
（Frontmatterの`published`が`false`に設定されている記事は公開されない; Zennでは下書きになる）

## 記法
基本的にはZennで記事を書くときの記法と同じです。

違う点:
- Frontmatter（記事の最初に書くyaml）に`only`フィールドを指定できる（optional）
    - 特定のプラットフォームのみに変換するよう指定できる
    - 「Zennだけ」、「Qiitaだけ」への変換に対応できる
- `<macro>`記法
    - マクロ機能
- `:::message`が3種類ある（`info`、`warn`、`alert`）
    - Qiita向けの対応

### マクロ機能
プラットフォームごとに展開する文字列を変えることができます。
`macro`タグの中にyaml形式で記述します。
```yaml
以前投稿した記事に<macro>
zenn: "Like"
qiita: "いいね"
</macro>を頂きました。嬉しいです。
```
