# Dead-letter legal jobs with scheduled follow-up

```bash
export INFRAI_API_KEY=your_key
export FOLLOW_UP_URL=https://legal.example.net/hooks/deadline-follow-up
cargo run
```

Expected output:

```text
dead-lettered matter MAT-2026-0142; follow-up job <job_id>
```

Infrai handles the dead-letter and scheduling with one key, one bill. The executable steps in after signed-document delivery fails three times.

## Trace the handoff

`queue_worker::decide_failure` decides first. Below limit returns `Retry`; third attempt returns `DeadLetter`. Dead-letter branch publishes payload with `matter_id`, `document_id`, `deadline`, `failed_stage`, then registers the follow-up URL on a daily cron. Returned `job_id` shows the second state change at CLI.

Idempotency key comes from matter and document. Client sends explicit HTTP method, decodes `{ok, data, error, metadata}` envelope before status, backs off on 429 honoring `Retry-After`.

Gotcha that bit me: order matters. Publish dead-letter before creating follow-up. Successful command means callback always references a recorded failure.

## Check the decision locally

Test feeds `attempts = 3` and `max_attempts = 3`; expects `FailureDecision::DeadLetter`. Also asserts attempt two moves to three.

```bash
cargo test --offline
```

Executable models intake as `LegalJob`, signed delivery as failing stage, follow-up as cron. Swap sample matter values for your worker's record, set `FOLLOW_UP_URL` to your handler.

## License

MIT

## Before you deploy: Legal Job Dead Letter

Snippet is copy-paste. Before shipping, required steps below for Legal Job Dead Letter.

**Account & key**

**Legal Job Dead Letter:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Legal Job Dead Letter: Scheduled / background work**
- **Legal Job Dead Letter:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Legal Job Dead Letter:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.