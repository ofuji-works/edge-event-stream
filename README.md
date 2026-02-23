# edge-event-stream 実装プロセス（MVP）

## 目的
Cloudflare Workers / Durable Objects / R2 を用いて、工場センシングの raw イベントを取り込み、
順序付きで保存し、簡易アルゴリズム（しきい値）で派生イベントを生成し、メトリクスDB + Metabase で可視化する。

## MVPの完成条件（DoD）
- [ ] デバイス（シミュレータ）から `POST /ingest` でイベントを送信できる
- [ ] 認証（MVP: APIキー）が通らないと 401 になる
- [ ] Stream DO が `seq` を採番し、R2に raw ログ（JSONL）を保存する
- [ ] しきい値判定により `anomaly` が生成される
- [ ] メトリクスDBに時系列データが入る
- [ ] Metabaseで温度グラフと異常一覧を表示できる
- [ ] E2E（送信→保存→可視化）が通る

## フェーズ0：リポジトリ初期化
### 0.1 リポジトリ作成
- [ ] GitHubに `edge-event-stream` を作成
- [ ] LICENSE（MIT/Apache-2.0）を追加
- [ ] README.md を追加
- [ ] `.gitignore`（Rust + Node + CF）を追加

### 0.2 Cargo workspace 作成
- [ ] ルートに `Cargo.toml`（workspace）
- [ ] `crates/*` を作成し、最低限ビルドが通ることを確認

---

## フェーズ1：Cloudflareセットアップ
### 1.1 Wranglerセットアップ
- [ ] `wrangler.toml` を作成
- [ ] アカウント/zone/worker名を設定
- [ ] ローカル実行（`wrangler dev`）が動くことを確認

### 1.2 リソース作成
- [ ] R2バケット作成（例: `edge-event-stream-raw`）
- [ ] KV namespace 作成（例: `EDGE_EVENT_STREAM_DEVICES`）
- [ ] Durable Object バインド設定（`StreamDO`）
- [ ] （任意）Queue 作成（MVPで後段処理を非同期にするなら）

---

## フェーズ2：コアモデルとport（trait）
### 2.1 Ingestのデータ構造
- [ ] `crates/core/src/ingest.rs` に `IngestRequest` / `IngestEvent` を定義
- [ ] 必須/任意の区別を決める
  - 必須: `device_id`, `batch_id`, `sent_at_ms`, `events[]`
  - イベント必須: `event_id`, `event_type`, `ts_device_ms`, `payload`
  - 任意: `stream_key`, `local_seq`, `sensor`, `headers`, `meta`

### 2.2 Port（trait）の定義
- [ ] `KvStore` trait（デバイス台帳参照用）
- [ ] `ObjectStore` trait（R2保存用）
- [ ] `MetricsSink` trait（メトリクス書き込み用）

---

## フェーズ3：adapters（Cloudflare実装）
### 3.1 KV adapter
- [ ] `adapters/cloudflare/kv.rs` に KV read を実装
- [ ] `device_id -> DeviceProvisioningRecord` を取得できるようにする
- [ ] MVPでは APIキー照合に必要な情報だけでOK
  - `status`, `api_key_hash` 等

### 3.2 R2 adapter
- [ ] `adapters/cloudflare/r2.rs` に R2 put を実装
- [ ] JSONLのオブジェクト保存ができること
- [ ] キー構造を決める
  - `stream/{stream_key}/seg/{start}-{end}.jsonl`

---

## フェーズ4：ingest-worker（入口）
### 4.1 API設計
- [ ] `POST /ingest`
- [ ] Headers:
  - `x-device-id`
  - `x-api-key`（MVP）
- [ ] Body: `IngestRequest`

### 4.2 検証・補完
- [ ] サイズ制限（上限を決める）
- [ ] JSON parse
- [ ] device_id と header の一致チェック（方針次第）
- [ ] `ts_ingest_ms` 付与
- [ ] `stream_key` 解決（無ければ `device_id` などから生成）
- [ ] request_id/trace_id の生成（任意）

### 4.3 認証（MVP: APIキー）
- [ ] KVから provisioning record を取得
- [ ] status=active を確認
- [ ] `x-api-key` を hash して一致確認（生キー保存しない）

### 4.4 Stream DO へ append
- [ ] `DO.fetch(append)` を呼ぶ
- [ ] 応答として `accepted_count` / `first_seq` / `last_seq` 等を返す

---

## フェーズ5：stream-do（順序・採番・R2保存）
### 5.1 DOの責務
- [ ] `seq` 採番（単調増加）
- [ ] `event_id` の重複排除（最近N件でもOK）
- [ ] in-memory buffer にイベントを貯める
- [ ] flush 条件:
  - 件数（例: 1000）
  - サイズ（例: 1MB）
  - 時間（例: 5秒）※Alarmで実現

### 5.2 R2へflush
- [ ] JSONL生成（1行1イベント）
- [ ] `stream/{stream_key}/seg/{start}-{end}.jsonl` に保存
- [ ] 成功したら `last_flushed_seq` を更新
- [ ] セグメント index を DO storage に保存

---

## フェーズ6：processor-worker（派生処理とDB）
> MVPでは「Stream DO が flush 後に Queue に通知」または「後でバッチ処理」でもOK

### 6.1 入力
- [ ] Queue message:
  - `stream_key`
  - `seg_key`（R2オブジェクトキー）
  - `seq_start`, `seq_end`

### 6.2 アルゴリズム（MVP）
- [ ] しきい値（温度 > threshold）
- [ ] anomaly event を生成（type: `anomaly.detected`）

### 6.3 メトリクスDBへ書き込み
- [ ] `device_metrics` に温度をinsert
- [ ] `anomaly_events` に異常をinsert

---

## フェーズ7：Metabase可視化
### 7.1 Metabase
- [ ] Metabase起動（Docker等）
- [ ] DB接続

### 7.2 ダッシュボード
- [ ] 温度の時系列グラフ
- [ ] 異常一覧（時間/設備/値）

---

## フェーズ8：device-simulator（送信確認）
### 8.1 CLI
- [ ] `--device-id`
- [ ] `--api-key`
- [ ] `--endpoint`
- [ ] 温度をランダム生成して送信
- [ ] 異常温度を混ぜるモード

---

## フェーズ9：E2Eテスト
- [ ] 正常データを送って、R2に保存されること
- [ ] 異常データを送って、DBに anomaly が入ること
- [ ] Metabaseで見えること
- [ ] 認証失敗が 401 になること

---

# MVP後のバックログ（優先度順）

## P0（運用前に必須）
- [ ] 署名認証（Ed25519）
- [ ] mTLS対応（Client Certificates / API Shield）
- [ ] nonce + timestamp でリプレイ防止（DeviceDO）
- [ ] DLQ（壊れた入力・失敗処理をR2へ隔離）
- [ ] idempotency（batch_id/event_id）の厳密化

## P1（プロダクトとして成立）
- [ ] アルゴリズム設定（enabled/params）
- [ ] スコープ解決（tenant/site/line/machine/sensor）
- [ ] EMA / Robust Z / CUSUM
- [ ] Slack/Email通知

## P2（SaaS化・スケール）
- [ ] マルチテナント分離（tenant_id）
- [ ] consumer group / offset
- [ ] リプレイ機能（range指定で再処理）
- [ ] コストメトリクス（課金指標）

## P3（高度）
- [ ] Wasmプラグインでアルゴリズム差し替え
- [ ] MLモデル配布
- [ ] 自動しきい値調整
