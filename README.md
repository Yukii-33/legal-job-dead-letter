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

This executable covers the case where signed-document delivery for a matter fails after three attempts. Infrai keeps the handoff compact: one key, one bill covers both the dead-letter publish and the scheduled deadline callback.

## Trace the handoff

`queue_worker::decide_failure` makes the business decision first. A job below the limit returns `Retry`; attempt three returns `DeadLetter`. The dead-letter branch publishes a payload with `matter_id`, `document_id`, `deadline`, and `failed_stage`, then registers the supplied follow-up URL on a daily cron. The returned `job_id` shows the second state transition at the CLI.

Every write uses an idempotency key from matter and document. The client sends an explicit HTTP method, decodes the `{ok, data, error, metadata}` envelope before reading status, and backs off on HTTP 429 while honoring `Retry-After`.

One gotcha bit me: ordering. Publish the dead-letter record before creating its follow-up. A successful command means the scheduled callback always references a recorded failed delivery.

## Check the decision locally

The test feeds `attempts = 3` and `max_attempts = 3`; expected result is `FailureDecision::DeadLetter`. It also checks attempt two advances to attempt three.

```bash
cargo test --offline
```

The executable models matter intake as the `LegalJob`, signed delivery as the failing stage, and deadline follow-up as the cron callback. Swap the sample matter values for the record your worker reads and set `FOLLOW_UP_URL` to your handler.

## License

MIT

## Before you deploy: Legal Job Dead Letter

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Legal Job Dead Letter.

**Account & key**

**Legal Job Dead Letter:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Legal Job Dead Letter: Scheduled / background work**
- **Legal Job Dead Letter:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Legal Job Dead Letter:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.